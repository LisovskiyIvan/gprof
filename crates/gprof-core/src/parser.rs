use crate::gl_enum;
use crate::model::*;
use memmap2::Mmap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;
use std::fs::File;
use std::path::Path;

#[derive(Default, Clone)]
struct RawTexData {
    width: u32,
    height: u32,
    format: u32,
    internal_format: u32,
    r#type: u32,
    length: usize,
    is_compressed: bool,
    target: String,
}

pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ParsedCapture, Box<dyn std::error::Error>> {
    let p = path.as_ref();
    let file = File::open(p)?;
    let mmap = unsafe { Mmap::map(&file)? };

    if (mmap.len() >= 2 && mmap[0] == 0x1f && mmap[1] == 0x8b) || p.extension().and_then(|s| s.to_str()) == Some("gz") {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut decoder = GzDecoder::new(&mmap[..]);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        parse_slice(&buf)
    } else {
        parse_slice(&mmap)
    }
}

pub fn parse_slice(slice: &[u8]) -> Result<ParsedCapture, Box<dyn std::error::Error>> {
    let root: Value = serde_json::from_slice(slice)?;

    // Canvas
    let mut canvas = CanvasInfo::default();
    if let Some(c) = root.get("canvas") {
        canvas.width = c.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.height = c.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.client_width = c.get("clientWidth").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.client_height = c.get("clientHeight").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.browser_agent = c.get("browserAgent").and_then(|v| v.as_str()).map(|s| s.to_string());
    }

    // Context
    let mut context = ContextInfo::default();
    if let Some(ctx) = root.get("context") {
        context.version = ctx.get("version").and_then(|v| v.as_u64()).map(|v| v as u32);
        if let Some(caps) = ctx.get("capabilities") {
            context.renderer = caps.get("RENDERER").and_then(|v| v.as_str()).map(|s| s.to_string());
            context.vendor = caps.get("VENDOR").and_then(|v| v.as_str()).map(|s| s.to_string());
            context.shading_language_version = caps.get("SHADING_LANGUAGE_VERSION").and_then(|v| v.as_str()).map(|s| s.to_string());
            context.max_texture_size = caps.get("MAX_TEXTURE_SIZE").and_then(|v| v.as_u64()).map(|v| v as u32);
            context.max_draw_buffers = caps.get("MAX_DRAW_BUFFERS").and_then(|v| v.as_u64()).map(|v| v as u32);
        }
        if let Some(comp) = ctx.get("compressedTextures").and_then(|c| c.get("COMPRESSED_TEXTURE_FORMATS")).and_then(|f| f.as_array()) {
            for fmt in comp {
                if let Some(s) = fmt.as_str() {
                    context.compressed_formats.push(s.to_string());
                }
            }
        }
    }

    // 1. Collect all textures across the JSON
    let mut raw_textures: FxHashMap<u32, RawTexData> = FxHashMap::default();
    collect_textures(&root, &mut raw_textures);

    // 2. Iterate commands and reconstruct WebGL state
    let mut summary = FrameCaptureSummary::default();
    let mut state_changes = StateChangeSummary::default();
    let mut draws = Vec::new();
    let mut uploads = Vec::new();
    let mut buffer_uploads = Vec::new();
    let mut seen_buffer_samples: FxHashSet<String> = FxHashSet::default();
    let mut passes = Vec::new();

    let init_state = root.get("initState");
    let mut current_depth_mask = init_state
        .and_then(|s| s.get("DepthState"))
        .and_then(|d| d.get("DEPTH_WRITEMASK"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let mut current_depth_test = init_state
        .and_then(|s| s.get("DepthState"))
        .and_then(|d| d.get("DEPTH_TEST"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let mut current_blend_enabled = init_state
        .and_then(|s| s.get("BlendState"))
        .and_then(|b| b.get("BLEND"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut current_cull_enabled = init_state
        .and_then(|s| s.get("CullState"))
        .and_then(|c| c.get("CULL_FACE"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut current_color_mask: [bool; 4] = [true, true, true, true];
    let mut current_vao: Option<u64> = None;

    let mut current_unit: u32 = 0;
    let mut bound_textures: FxHashMap<u32, u32> = FxHashMap::default(); // unit -> tex_id
    let mut current_program_id: Option<u32> = None;
    let mut current_fbo: String = "Default Backbuffer".to_string();

    let mut tex_draw_counts: FxHashMap<u32, usize> = FxHashMap::default();
    let mut tex_uniform_names: FxHashMap<u32, FxHashSet<String>> = FxHashMap::default();
    let mut tex_bound_units: FxHashMap<u32, FxHashSet<u32>> = FxHashMap::default();

    let mut program_draw_counts: FxHashMap<u32, usize> = FxHashMap::default();
    let mut program_shaders: FxHashMap<u32, (String, Vec<String>)> = FxHashMap::default();

    // Pass tracking
    let mut current_viewport: Option<(u32, u32)> = None;
    let mut current_pass = RenderPassItem {
        pass_index: 0,
        fbo_name: current_fbo.clone(),
        pass_type: "Init".to_string(),
        draw_count: 0,
        triangles: 0,
        points: 0,
        shaders: Vec::new(),
        start_command: 0,
        viewport_width: None,
        viewport_height: None,
        vram_bytes: 0,
    };

    let empty_vec = Vec::new();
    let commands = root.get("commands").and_then(|c| c.as_array()).unwrap_or(&empty_vec);
    summary.total_commands = commands.len();

    let mut particle_systems: Vec<ParticleSystemItem> = Vec::new();

    for (cmd_idx, cmd) in commands.iter().enumerate() {
        let name = cmd.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let text = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let args = cmd.get("commandArguments").and_then(|v| v.as_array());

        match name {
            "depthMask" => {
                state_changes.depth_mask_calls += 1;
                if let Some(args) = args {
                    if let Some(val) = args.first().and_then(|v| v.as_bool()) {
                        if val == current_depth_mask {
                            state_changes.redundant_depth_mask += 1;
                        }
                        current_depth_mask = val;
                    }
                }
            }
            "colorMask" => {
                state_changes.color_mask_calls += 1;
                if let Some(args) = args {
                    if args.len() >= 4 {
                        let r = args[0].as_bool().unwrap_or(true);
                        let g = args[1].as_bool().unwrap_or(true);
                        let b = args[2].as_bool().unwrap_or(true);
                        let a = args[3].as_bool().unwrap_or(true);
                        if current_color_mask == [r, g, b, a] {
                            state_changes.redundant_color_mask += 1;
                        }
                        current_color_mask = [r, g, b, a];
                    }
                }
            }
            "enable" => {
                if let Some(args) = args {
                    if let Some(cap) = args.first().and_then(|v| v.as_u64()) {
                        match cap {
                            3042 => {
                                state_changes.blend_toggle_calls += 1;
                                if current_blend_enabled {
                                    state_changes.redundant_blend_toggles += 1;
                                }
                                current_blend_enabled = true;
                            }
                            2884 => {
                                state_changes.cull_toggle_calls += 1;
                                if current_cull_enabled {
                                    state_changes.redundant_cull_toggles += 1;
                                }
                                current_cull_enabled = true;
                            }
                            2929 => {
                                state_changes.depth_test_calls += 1;
                                if current_depth_test {
                                    state_changes.redundant_depth_test += 1;
                                }
                                current_depth_test = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
            "disable" => {
                if let Some(args) = args {
                    if let Some(cap) = args.first().and_then(|v| v.as_u64()) {
                        match cap {
                            3042 => {
                                state_changes.blend_toggle_calls += 1;
                                if !current_blend_enabled {
                                    state_changes.redundant_blend_toggles += 1;
                                }
                                current_blend_enabled = false;
                            }
                            2884 => {
                                state_changes.cull_toggle_calls += 1;
                                if !current_cull_enabled {
                                    state_changes.redundant_cull_toggles += 1;
                                }
                                current_cull_enabled = false;
                            }
                            2929 => {
                                state_changes.depth_test_calls += 1;
                                if !current_depth_test {
                                    state_changes.redundant_depth_test += 1;
                                }
                                current_depth_test = false;
                            }
                            _ => {}
                        }
                    }
                }
            }
            "bindVertexArray" => {
                state_changes.vao_bind_calls += 1;
                let vao_id = args.and_then(|a| a.first()).and_then(|v| {
                    v.get("__SPECTOR_Object_TAG")
                        .and_then(|t| t.get("id"))
                        .and_then(|id| id.as_u64())
                });
                if vao_id == current_vao && vao_id.is_some() {
                    state_changes.redundant_vao_binds += 1;
                }
                current_vao = vao_id;
            }
            "activeTexture" => {
                if let Some(args) = args {
                    if let Some(arg0) = args.first().and_then(|v| v.as_u64()) {
                        current_unit = if arg0 >= 33984 { (arg0 - 33984) as u32 } else { arg0 as u32 };
                    }
                }
            }
            "bindTexture" => {
                summary.texture_bind_calls += 1;
                state_changes.texture_bind_calls += 1;
                if let Some(args) = args {
                    if args.len() >= 2 {
                        if let Some(tid) = extract_spektor_id(&args[1]) {
                            if bound_textures.get(&current_unit) == Some(&tid) {
                                state_changes.redundant_texture_binds += 1;
                            }
                            bound_textures.insert(current_unit, tid);
                            tex_bound_units.entry(tid).or_default().insert(current_unit);
                        }
                    }
                }
            }
            "bindFramebuffer" => {
                summary.fbo_bind_calls += 1;
                state_changes.fbo_bind_calls += 1;
                let new_fbo = if let Some(args) = args {
                    if args.len() >= 2 {
                        if let Some(tag) = args[1].get("__SPECTOR_Object_TAG") {
                            tag.get("displayText")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    tag.get("id")
                                        .and_then(|v| v.as_u64())
                                        .map(|id| format!("FBO_{}", id))
                                        .unwrap_or_else(|| "Unknown FBO".to_string())
                                })
                        } else {
                            "Default Backbuffer".to_string()
                        }
                    } else {
                        "Default Backbuffer".to_string()
                    }
                } else {
                    "Default Backbuffer".to_string()
                };

                if new_fbo == current_fbo {
                    state_changes.redundant_fbo_binds += 1;
                } else {
                    if current_pass.draw_count > 0 {
                        passes.push(current_pass);
                        current_pass = RenderPassItem {
                            pass_index: passes.len(),
                            fbo_name: new_fbo.clone(),
                            pass_type: classify_pass(&new_fbo, &[]),
                            draw_count: 0,
                            triangles: 0,
                            points: 0,
                            shaders: Vec::new(),
                            start_command: cmd_idx,
                            viewport_width: current_viewport.map(|v| v.0),
                            viewport_height: current_viewport.map(|v| v.1),
                            vram_bytes: current_viewport.map(|v| (v.0 as usize) * (v.1 as usize) * 8).unwrap_or(0),
                        };
                    } else {
                        current_pass.fbo_name = new_fbo.clone();
                    }
                    current_fbo = new_fbo;
                }
            }
            "viewport" => {
                if let Some(args) = args {
                    if args.len() >= 4 {
                        let w = args[2].as_u64().unwrap_or(0) as u32;
                        let h = args[3].as_u64().unwrap_or(0) as u32;
                        current_viewport = Some((w, h));
                        current_pass.viewport_width = Some(w);
                        current_pass.viewport_height = Some(h);
                        current_pass.vram_bytes = (w as usize) * (h as usize) * 8;
                    }
                }
            }
            "useProgram" => {
                summary.use_program_calls += 1;
                state_changes.use_program_calls += 1;
                if let Some(args) = args {
                    if let Some(first) = args.first() {
                        let pid = extract_spektor_id(first);
                        if pid == current_program_id && pid.is_some() {
                            summary.redundant_use_program += 1;
                            state_changes.redundant_use_program += 1;
                        }
                        current_program_id = pid;
                    }
                }
            }
            "texImage2D" | "texSubImage2D" => {
                summary.dynamic_uploads_count += 1;
                let target = args.and_then(|a| a.first()).and_then(|v| v.as_u64()).unwrap_or(3553) as u32;
                let width = args.and_then(|a| a.get(3)).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let height = args.and_then(|a| a.get(4)).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let format = args.and_then(|a| a.get(6)).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let r#type = args.and_then(|a| a.get(7)).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                let mut stack_trace = Vec::new();
                if let Some(st) = cmd.get("stackTrace").and_then(|v| v.as_array()) {
                    for line in st {
                        if let Some(s) = line.as_str() {
                            stack_trace.push(s.to_string());
                        }
                    }
                }

                uploads.push(DynamicUploadItem {
                    command_index: cmd_idx,
                    name: name.to_string(),
                    target,
                    width,
                    height,
                    format,
                    r#type,
                    stack_trace,
                });
            }
            "bufferSubData" | "bufferData" => {
                if name == "bufferSubData" {
                    summary.buffer_subdata_calls += 1;
                }
                let target = args.and_then(|a| a.first()).and_then(|v| v.as_u64()).unwrap_or(34962) as u32;
                let target_name = match target {
                    34962 => "ARRAY_BUFFER".to_string(),
                    34963 => "ELEMENT_ARRAY_BUFFER".to_string(),
                    35051 => "UNIFORM_BUFFER".to_string(),
                    35882 => "TRANSFORM_FEEDBACK_BUFFER".to_string(),
                    _ => format!("BUFFER_0x{:X}", target),
                };
                let offset = args.and_then(|a| a.get(1)).and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                let mut size_bytes = 0;
                let mut sample_str = None;
                if let Some(data_val) = args.and_then(|a| a.get(2)) {
                    if let Some(arr) = data_val.as_array() {
                        size_bytes = arr.len() * 4;
                        let preview: Vec<String> = arr.iter().take(4).map(|v| v.to_string()).collect();
                        sample_str = Some(preview.join(", "));
                    } else if let Some(obj) = data_val.as_object() {
                        size_bytes = obj.len() * 4;
                        let preview: Vec<String> = obj.values().take(4).map(|v| v.to_string()).collect();
                        sample_str = Some(preview.join(", "));
                    }
                }
                if size_bytes == 0 && text.contains("[..(") {
                    if let Some(pos) = text.find("[..(") {
                        if let Some(end) = text[pos..].find(")..]") {
                            let num_str = &text[pos + 4..pos + end];
                            if let Ok(count) = num_str.parse::<usize>() {
                                size_bytes = count * 4;
                            }
                        }
                    }
                }

                let mut stack_trace = Vec::new();
                if let Some(st) = cmd.get("stackTrace").and_then(|v| v.as_array()) {
                    for line in st {
                        if let Some(s) = line.as_str() {
                            stack_trace.push(s.to_string());
                        }
                    }
                }

                let sample_key = format!("{}:{}:{}", target, offset, sample_str.as_deref().unwrap_or(""));
                let is_duplicate = seen_buffer_samples.contains(&sample_key);
                if !sample_key.is_empty() {
                    seen_buffer_samples.insert(sample_key);
                }
                if is_duplicate {
                    summary.duplicate_buffer_uploads += 1;
                }

                buffer_uploads.push(DynamicBufferUploadItem {
                    command_index: cmd_idx,
                    name: name.to_string(),
                    target,
                    target_name,
                    offset,
                    size_bytes,
                    stack_trace,
                    data_sample: sample_str,
                    is_duplicate,
                });
            }
            _ => {}
        }

        // Check if it's a draw command
        let is_draw = name.starts_with("draw");
        if is_draw {
            summary.draw_calls += 1;
            let dc = cmd.get("DrawCall");

            // Extract mode and counts
            let mut mode = 4; // TRIANGLES default
            let mut count = 0;
            let mut instance_count = 1;
            let mut index_type = None;
            let mut index_type_name = None;

            if let Some(args) = args {
                if name.contains("Elements") {
                    mode = args.first().and_then(|v| v.as_u64()).unwrap_or(4) as u32;
                    count = args.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    if let Some(itype) = args.get(2).and_then(|v| v.as_u64()) {
                        let itype_u32 = itype as u32;
                        index_type = Some(itype_u32);
                        match itype_u32 {
                            5123 => {
                                summary.index_16bit_draws += 1;
                                index_type_name = Some("UNSIGNED_SHORT (16-bit)".to_string());
                            }
                            5125 => {
                                summary.index_32bit_draws += 1;
                                index_type_name = Some("UNSIGNED_INT (32-bit)".to_string());
                            }
                            5121 => {
                                index_type_name = Some("UNSIGNED_BYTE (8-bit)".to_string());
                            }
                            _ => {
                                index_type_name = Some(format!("0x{:X}", itype_u32));
                            }
                        }
                    }
                    if name.contains("Instanced") && args.len() > 4 {
                        instance_count = args[4].as_u64().unwrap_or(1) as u32;
                    }
                    if name.contains("Instanced") {
                        summary.draw_elements_instanced += 1;
                    } else {
                        summary.draw_elements += 1;
                    }
                } else if name.contains("Arrays") {
                    mode = args.first().and_then(|v| v.as_u64()).unwrap_or(4) as u32;
                    count = args.get(2).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    if name.contains("Instanced") && args.len() > 3 {
                        instance_count = args[3].as_u64().unwrap_or(1) as u32;
                    }
                    if name.contains("Instanced") {
                        summary.draw_arrays_instanced += 1;
                    } else {
                        summary.draw_arrays += 1;
                    }
                }
            }

            if current_blend_enabled {
                summary.blended_draws += 1;
            } else {
                summary.opaque_draws += 1;
            }

            let mut triangles = 0;
            let mut points = 0;
            let mut lines = 0;

            match mode {
                4 => triangles = ((count / 3) * instance_count) as usize,
                5 => triangles = (count.saturating_sub(2) * instance_count) as usize,
                0 => points = (count * instance_count) as usize,
                1 => lines = ((count / 2) * instance_count) as usize,
                2 | 3 => lines = (count * instance_count) as usize,
                _ => {}
            }

            summary.triangles += triangles;
            summary.points += points;
            summary.lines += lines;

            // Shaders & Program
            let mut shader_name = "unknown".to_string();
            let mut defines = Vec::new();
            if let Some(dc_obj) = dc {
                if let Some(shaders) = dc_obj.get("shaders").and_then(|v| v.as_array()) {
                    if let Some(vs) = shaders.first().and_then(|s| s.get("source")).and_then(|s| s.as_str()) {
                        for line in vs.lines() {
                            if line.contains("#define SHADER_NAME") {
                                shader_name = line.replace("#define SHADER_NAME", "").trim().to_string();
                            } else if line.starts_with("#define ") {
                                defines.push(line.trim().to_string());
                            }
                        }
                    }
                }
            }

            if let Some(pid) = current_program_id {
                *program_draw_counts.entry(pid).or_default() += 1;
                program_shaders.entry(pid).or_insert_with(|| (shader_name.clone(), defines.clone()));
            }

            // Uniforms inspection (glowColor, world matrix, bound textures)
            let mut is_glow_pass = false;
            let mut is_black_glow = false;
            let mut glow_color = None;
            let mut world_position = None;

            if shader_name.contains("glowMapGeneration") {
                is_glow_pass = true;
                summary.glow_total_draws += 1;
                summary.glow_total_triangles += triangles;
            }

            let mut has_emissive_tex = false;
            if let Some(dc_obj) = dc {
                if let Some(uniforms) = dc_obj.get("uniforms").and_then(|u| u.as_array()) {
                    for u in uniforms {
                        let uname = u.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if uname == "glowColor" {
                            if let Some(arr) = u.get("value").and_then(|v| v.as_array()) {
                                if arr.len() >= 3 {
                                    let r = arr[0].as_f64().unwrap_or(0.0) as f32;
                                    let g = arr[1].as_f64().unwrap_or(0.0) as f32;
                                    let b = arr[2].as_f64().unwrap_or(0.0) as f32;
                                    let a = arr.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                                    glow_color = Some([r, g, b, a]);
                                }
                            }
                        } else if uname == "world" {
                            if let Some(arr) = u.get("value").and_then(|v| v.as_array()) {
                                if arr.len() >= 16 {
                                    let x = arr[12].as_f64().unwrap_or(0.0) as f32;
                                    let y = arr[13].as_f64().unwrap_or(0.0) as f32;
                                    let z = arr[14].as_f64().unwrap_or(0.0) as f32;
                                    world_position = Some([x, y, z]);
                                }
                            }
                        }

                        if uname.to_lowercase().contains("emissive") && u.get("value").map(|v| v.is_object()).unwrap_or(false) {
                            has_emissive_tex = true;
                        }

                        // Check samplers mapping to texture units
                        if let Some(val) = u.get("value") {
                            if let Some(unit_idx) = val.as_u64() {
                                if let Some(&tid) = bound_textures.get(&(unit_idx as u32)) {
                                    *tex_draw_counts.entry(tid).or_default() += 1;
                                    tex_uniform_names.entry(tid).or_default().insert(uname.to_string());
                                }
                            } else if let Some(tid) = extract_spektor_id(val) {
                                *tex_draw_counts.entry(tid).or_default() += 1;
                                tex_uniform_names.entry(tid).or_default().insert(uname.to_string());
                            }
                        }
                    }
                }
            }

            if is_glow_pass {
                let is_black = match glow_color {
                    Some([r, g, b, _]) => r == 0.0 && g == 0.0 && b == 0.0,
                    None => true,
                };
                if is_black && !has_emissive_tex {
                    is_black_glow = true;
                    summary.glow_black_draws += 1;
                    summary.glow_black_triangles += triangles;
                } else {
                    summary.glow_active_draws += 1;
                    summary.glow_active_triangles += triangles;
                }
            }

            // Attributes / vertex count
            let mut vertex_buffer_bytes = 0;
            let mut vertex_count_estimate = 0;
            if let Some(dc_obj) = dc {
                if let Some(attrs) = dc_obj.get("attributes").and_then(|a| a.as_array()) {
                    for attr in attrs {
                        let name = attr.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let blen = attr.get("bufferLength").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        if name == "position" {
                            vertex_buffer_bytes = blen;
                            vertex_count_estimate = blen / 12; // 3 floats = 12 bytes
                        }
                    }
                }
            }

            let draw_item_idx = draws.len();

            // Track particles
            if shader_name.contains("gpuUpdateParticles") {
                particle_systems.push(ParticleSystemItem {
                    update_draw_index: draw_item_idx,
                    render_draw_index: None,
                    particle_count: count as usize,
                    quads_rendered: (count as usize) * 2,
                    is_transform_feedback: true,
                });
            } else if shader_name.contains("gpuRenderParticles") {
                if let Some(last) = particle_systems.last_mut() {
                    if last.render_draw_index.is_none() {
                        last.render_draw_index = Some(draw_item_idx);
                    }
                }
            }

            let draw_item = DrawCallItem {
                index: draw_item_idx,
                command_id: cmd_idx,
                name: name.to_string(),
                text: text.to_string(),
                mode,
                mode_name: gl_enum::primitive_mode_name(mode).to_string(),
                count,
                instance_count,
                triangles,
                points,
                lines,
                fbo_name: current_fbo.clone(),
                program_id: current_program_id,
                shader_name: shader_name.clone(),
                is_glow_pass,
                is_black_glow,
                glow_color,
                world_position,
                vertex_buffer_bytes,
                vertex_count_estimate,
                is_instanced: name.contains("Instanced"),
                index_type,
                index_type_name,
                blend_enabled: current_blend_enabled,
                depth_write: current_depth_mask,
                cull_enabled: current_cull_enabled,
            };

            // Update current pass
            current_pass.draw_count += 1;
            current_pass.triangles += triangles;
            current_pass.points += points;
            if !current_pass.shaders.contains(&shader_name) {
                current_pass.shaders.push(shader_name);
            }

            draws.push(draw_item);
        }
    }

    if current_pass.draw_count > 0 {
        current_pass.pass_type = classify_pass(&current_pass.fbo_name, &current_pass.shaders);
        passes.push(current_pass);
    }

    // Set pass types for all passes
    for p in &mut passes {
        p.pass_type = classify_pass(&p.fbo_name, &p.shaders);
    }

    // Compile textures list
    let mut textures = Vec::new();
    let mut total_texture_bytes = 0;
    for (tid, raw) in raw_textures {
        let draw_cnt = tex_draw_counts.get(&tid).copied().unwrap_or(0);
        let units: Vec<u32> = tex_bound_units.get(&tid).map(|s| s.iter().copied().collect()).unwrap_or_default();
        let unames: Vec<String> = tex_uniform_names.get(&tid).map(|s| s.iter().cloned().collect()).unwrap_or_default();
        let is_pot = raw.width > 0 && raw.height > 0 && raw.width.is_power_of_two() && raw.height.is_power_of_two();
        total_texture_bytes += raw.length;

        textures.push(TextureItem {
            id: tid,
            width: raw.width,
            height: raw.height,
            format: raw.format,
            format_name: gl_enum::format_name(raw.format).to_string(),
            internal_format: raw.internal_format,
            internal_format_name: gl_enum::format_name(raw.internal_format).to_string(),
            r#type: raw.r#type,
            type_name: gl_enum::type_name(raw.r#type).to_string(),
            length: raw.length,
            is_compressed: raw.is_compressed,
            target: raw.target,
            draw_count: draw_cnt,
            bound_units: units,
            uniform_names: unames,
            is_pot,
        });
    }

    textures.sort_by(|a, b| b.length.cmp(&a.length));
    summary.total_texture_bytes = total_texture_bytes;
    summary.total_textures_count = textures.len();
    summary.unique_programs = program_draw_counts.len();

    // Compile programs
    let mut programs = Vec::new();
    for (pid, cnt) in program_draw_counts {
        let (sname, defs) = program_shaders.remove(&pid).unwrap_or_else(|| ("unknown".to_string(), Vec::new()));
        let is_pbr = sname.contains("pbr");
        programs.push(ProgramItem {
            id: pid,
            draw_count: cnt,
            shader_name: sname,
            defines: defs,
            is_pbr,
        });
    }
    programs.sort_by(|a, b| b.draw_count.cmp(&a.draw_count));

    summary.particle_systems_count = particle_systems.len();
    summary.total_particles = particle_systems.iter().map(|p| p.particle_count).sum();

    // Count active FBOs
    let active_fbos: FxHashSet<String> = passes.iter().map(|p| p.fbo_name.clone()).collect();
    summary.active_fbo_count = active_fbos.len();

    state_changes.total_redundant_calls = state_changes.redundant_depth_mask
        + state_changes.redundant_color_mask
        + state_changes.redundant_blend_toggles
        + state_changes.redundant_cull_toggles
        + state_changes.redundant_depth_test
        + state_changes.redundant_fbo_binds
        + state_changes.redundant_texture_binds
        + state_changes.redundant_vao_binds
        + state_changes.redundant_use_program;

    let mut memory_summary = MemorySummary::default();
    if let Some(mem) = root.get("memory") {
        if let Some(buf_obj) = mem.get("Buffer").and_then(|v| v.as_object()) {
            let max_val = buf_obj.values().filter_map(|v| v.as_u64()).max().unwrap_or(0);
            memory_summary.buffer_memory_bytes = max_val as usize;
        }
        if let Some(tex_obj) = mem.get("Texture2d").and_then(|v| v.as_object()) {
            let max_val = tex_obj.values().filter_map(|v| v.as_u64()).max().unwrap_or(0);
            memory_summary.texture_memory_bytes = max_val as usize;
        }
        if let Some(prog_obj) = mem.get("Program").and_then(|v| v.as_object()) {
            let max_val = prog_obj.values().filter_map(|v| v.as_u64()).max().unwrap_or(0);
            memory_summary.program_memory_bytes = max_val as usize;
        }
        if let Some(rb_obj) = mem.get("Renderbuffer").and_then(|v| v.as_object()) {
            let max_val = rb_obj.values().filter_map(|v| v.as_u64()).max().unwrap_or(0);
            memory_summary.renderbuffer_memory_bytes = max_val as usize;
        }
    }

    Ok(ParsedCapture {
        canvas: Some(canvas),
        context: Some(context),
        summary,
        state_changes,
        memory: memory_summary,
        textures,
        draws,
        passes,
        programs,
        particles: particle_systems,
        uploads,
        buffer_uploads,
    })
}

fn classify_pass(fbo: &str, shaders: &[String]) -> String {
    let s_lower = shaders.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>().join(" ");
    if s_lower.contains("procedural") {
        "Procedural / VAT Animation".to_string()
    } else if s_lower.contains("glowmapgeneration") {
        "GlowLayer Generation".to_string()
    } else if s_lower.contains("kernelblur") {
        "Gaussian / Kernel Blur".to_string()
    } else if fbo.contains("Default") {
        if s_lower.contains("layer") && !s_lower.contains("pbr") {
            "UI / 2D Canvas Layer".to_string()
        } else if shaders.len() <= 3 && s_lower.contains("pbr") {
            "Overlay / Weapon Camera".to_string()
        } else {
            "Main Scene (Color + Depth)".to_string()
        }
    } else {
        format!("Offscreen RenderTarget ({})", fbo)
    }
}

fn extract_spektor_id(v: &Value) -> Option<u32> {
    if let Some(tag) = v.get("__SPECTOR_Object_TAG") {
        tag.get("id").and_then(|id| id.as_u64()).map(|id| id as u32)
    } else {
        None
    }
}

fn collect_textures(v: &Value, map: &mut FxHashMap<u32, RawTexData>) {
    match v {
        Value::Object(obj) => {
            if let Some(tag) = obj.get("__SPECTOR_Object_TAG") {
                if tag.get("typeName").and_then(|t| t.as_str()) == Some("WebGLTexture") {
                    if let Some(id) = tag.get("id").and_then(|id| id.as_u64()).map(|id| id as u32) {
                        let mut item = RawTexData::default();
                        if let Some(cd) = obj.get("__SPECTOR_Object_CustomData") {
                            item.width = cd.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
                            item.height = cd.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;
                            item.format = cd.get("format").and_then(|f| f.as_u64()).unwrap_or(0) as u32;
                            item.internal_format = cd.get("internalFormat").and_then(|f| f.as_u64()).unwrap_or(0) as u32;
                            item.r#type = cd.get("type").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
                            item.length = cd.get("length").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
                            item.is_compressed = cd.get("isCompressed").and_then(|c| c.as_bool()).unwrap_or(false);
                            item.target = cd.get("target").and_then(|t| t.as_str()).unwrap_or("").to_string();
                        }
                        map.entry(id).and_modify(|existing| {
                            if item.length > existing.length {
                                *existing = item.clone();
                            }
                        }).or_insert(item);
                    }
                }
            }
            for val in obj.values() {
                collect_textures(val, map);
            }
        }
        Value::Array(arr) => {
            for val in arr {
                collect_textures(val, map);
            }
        }
        _ => {}
    }
}
