mod cmd;
mod helpers;

use cmd::Cmd;
use helpers::{classify_pass, collect_textures, compile_programs, compile_textures, extract_spektor_id, RawTexData};

use crate::gl_enum;
use crate::model::*;
use memmap2::Mmap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::Path;

pub fn parse_slice(slice: &[u8]) -> Result<ParsedCapture, Box<dyn std::error::Error>> {
    let root: CaptureRoot = serde_json::from_slice(slice)?;
    build_capture(root)
}

pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ParsedCapture, Box<dyn std::error::Error>> {
    let p = path.as_ref();
    let file = File::open(p)?;
    let mmap = unsafe { Mmap::map(&file)? };

    if (mmap.len() >= 2 && mmap[0] == 0x1f && mmap[1] == 0x8b) || p.extension().and_then(|s| s.to_str()) == Some("gz") {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let cap = if mmap.len() >= 4 && (mmap.len() as u64) < 4294967296 {
            let n = mmap.len();
            u32::from_le_bytes([mmap[n - 4], mmap[n - 3], mmap[n - 2], mmap[n - 1]]) as usize
        } else {
            0
        };
        let mut buf = Vec::with_capacity(cap);
        {
            let mut decoder = GzDecoder::new(&mmap[..]);
            decoder.read_to_end(&mut buf)?;
        }
        drop(mmap);
        parse_slice(&buf)
    } else {
        parse_slice(&mmap)
    }
}

#[derive(Deserialize)]
struct CaptureRoot<'a> {
    #[serde(default)]
    canvas: Option<Value>,
    #[serde(default)]
    context: Option<Value>,
    #[serde(default)]
    memory: Option<Value>,
    #[serde(default, rename = "initState")]
    init_state: Option<Value>,
    #[serde(default)]
    analyses: Option<Value>,
    #[serde(default, rename = "endState")]
    end_state: Option<Value>,
    #[serde(default, rename = "endTime")]
    end_time: Option<Value>,
    #[serde(default, rename = "frameMemory")]
    frame_memory: Option<Value>,
    #[serde(default, rename = "startTime")]
    start_time: Option<Value>,
    #[serde(default, rename = "listenCommandsStartTime")]
    listen_commands_start_time: Option<Value>,
    #[serde(default, rename = "listenCommandsEndTime")]
    listen_commands_end_time: Option<Value>,
    #[serde(borrow, default)]
    commands: Vec<&'a RawValue>,
}

fn build_capture<'a>(mut root: CaptureRoot<'a>) -> Result<ParsedCapture, Box<dyn std::error::Error>> {
    // Canvas
    let mut canvas = CanvasInfo::default();
    if let Some(c) = root.canvas.as_ref() {
        canvas.width = c.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.height = c.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.client_width = c.get("clientWidth").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.client_height = c.get("clientHeight").and_then(|v| v.as_u64()).map(|v| v as u32);
        canvas.browser_agent = c.get("browserAgent").and_then(|v| v.as_str()).map(|s| s.to_string());
    }

    // Context
    let mut context = ContextInfo::default();
    if let Some(ctx) = root.context.as_ref() {
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

    // 1. Collect textures from non-command root fields; per-command values collected in loop
    let mut raw_textures: FxHashMap<u32, RawTexData> = FxHashMap::default();
    if let Some(c) = root.canvas.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.context.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.init_state.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.memory.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.analyses.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.end_state.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.end_time.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.frame_memory.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.start_time.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.listen_commands_start_time.as_ref() {
        collect_textures(c, &mut raw_textures);
    }
    if let Some(c) = root.listen_commands_end_time.as_ref() {
        collect_textures(c, &mut raw_textures);
    }

    // 2. Iterate commands and reconstruct WebGL state
    let mut summary = FrameCaptureSummary::default();
    let mut state_changes = StateChangeSummary::default();
    let mut draws = Vec::new();
    let mut uploads = Vec::new();
    let mut buffer_uploads = Vec::new();
    let mut uniform_commands = Vec::new();
    let mut current_uniform_values: FxHashMap<(u32, u32), u64> = FxHashMap::default();
    let mut seen_buffer_samples: FxHashSet<u64> = FxHashSet::default();
    let mut val_buf = String::new();
    let mut passes = Vec::new();

    let init_state = root.init_state.as_ref();
    let drawing_buffer = (canvas.width.unwrap_or(0), canvas.height.unwrap_or(0));

    let mut current_viewport = Rect {
        x: 0,
        y: 0,
        width: drawing_buffer.0,
        height: drawing_buffer.1,
    };
    if let Some(vp) = init_state.and_then(|s| s.get("DrawState")).and_then(|d| d.get("VIEWPORT")).and_then(|v| v.as_array()) {
        if vp.len() >= 4 {
            let x = vp[0].as_i64().unwrap_or(0) as i32;
            let y = vp[1].as_i64().unwrap_or(0) as i32;
            let w = vp[2].as_u64().unwrap_or(drawing_buffer.0 as u64) as u32;
            let h = vp[3].as_u64().unwrap_or(drawing_buffer.1 as u64) as u32;
            current_viewport = Rect { x, y, width: w, height: h };
        }
    }

    let mut current_scissor = ScissorState::default();
    if let Some(sc) = init_state.and_then(|s| s.get("ScissorState")) {
        current_scissor.enabled = sc.get("SCISSOR_TEST").and_then(|v| v.as_bool()).unwrap_or(false);
        if let Some(sb) = sc.get("SCISSOR_BOX").and_then(|v| v.as_array()) {
            if sb.len() >= 4 {
                current_scissor.rect = Rect {
                    x: sb[0].as_i64().unwrap_or(0) as i32,
                    y: sb[1].as_i64().unwrap_or(0) as i32,
                    width: sb[2].as_u64().unwrap_or(0) as u32,
                    height: sb[3].as_u64().unwrap_or(0) as u32,
                };
            }
        }
    }

    let mut current_depth_mask = init_state
        .and_then(|s| s.get("DepthState"))
        .and_then(|d| d.get("DEPTH_WRITEMASK"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let mut current_depth_test = init_state.and_then(|s| s.get("DepthState")).and_then(|d| d.get("DEPTH_TEST")).and_then(|v| v.as_bool()).unwrap_or(true);
    let mut current_blend_enabled = init_state.and_then(|s| s.get("BlendState")).and_then(|b| b.get("BLEND")).and_then(|v| v.as_bool()).unwrap_or(false);
    let mut current_cull_enabled = init_state.and_then(|s| s.get("CullState")).and_then(|c| c.get("CULL_FACE")).and_then(|v| v.as_bool()).unwrap_or(false);
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
    let mut current_pass = RenderPassItem {
        pass_index: 0,
        fbo_name: current_fbo.clone(),
        pass_type: "Init".to_string(),
        draw_count: 0,
        triangles: 0,
        points: 0,
        shaders: Vec::new(),
        start_command: 0,
        viewport_width: Some(current_viewport.width),
        viewport_height: Some(current_viewport.height),
        drawing_buffer,
        scissor: if current_scissor.enabled { Some(current_scissor) } else { None },
        vram_bytes: (current_viewport.width as usize) * (current_viewport.height as usize) * 8,
    };

    summary.total_commands = root.commands.len();

    let commands = std::mem::take(&mut root.commands);

    let mut particle_systems: Vec<ParticleSystemItem> = Vec::new();

    for (cmd_idx, raw) in commands.into_iter().enumerate() {
        let cmd = serde_json::from_str::<Cmd<'_>>(raw.get())?;
        cmd.collect_textures(&mut raw_textures);
        let name = cmd.name;
        let text = cmd.text.as_deref().unwrap_or("");
        let args = cmd.command_arguments.as_deref();

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
                            3089 => {
                                current_scissor.enabled = true;
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
                            3089 => {
                                current_scissor.enabled = false;
                            }
                            _ => {}
                        }
                    }
                }
            }
            "scissor" => {
                if let Some(args) = args {
                    if args.len() >= 4 {
                        let x = args[0].as_i64().unwrap_or(0) as i32;
                        let y = args[1].as_i64().unwrap_or(0) as i32;
                        let w = args[2].as_u64().unwrap_or(0) as u32;
                        let h = args[3].as_u64().unwrap_or(0) as u32;
                        current_scissor.rect = Rect { x, y, width: w, height: h };
                    }
                }
            }
            "bindVertexArray" => {
                state_changes.vao_bind_calls += 1;
                let vao_id = args
                    .and_then(|a| a.first())
                    .and_then(|v| v.get("__SPECTOR_Object_TAG").and_then(|t| t.get("id")).and_then(|id| id.as_u64()));
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
                                .unwrap_or_else(|| tag.get("id").and_then(|v| v.as_u64()).map(|id| format!("FBO_{}", id)).unwrap_or_else(|| "Unknown FBO".to_string()))
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
                            pass_type: String::new(),
                            draw_count: 0,
                            triangles: 0,
                            points: 0,
                            shaders: Vec::new(),
                            start_command: cmd_idx,
                            viewport_width: Some(current_viewport.width),
                            viewport_height: Some(current_viewport.height),
                            drawing_buffer,
                            scissor: if current_scissor.enabled { Some(current_scissor) } else { None },
                            vram_bytes: (current_viewport.width as usize) * (current_viewport.height as usize) * 8,
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
                        let x = args[0].as_i64().unwrap_or(0) as i32;
                        let y = args[1].as_i64().unwrap_or(0) as i32;
                        let w = args[2].as_u64().unwrap_or(0) as u32;
                        let h = args[3].as_u64().unwrap_or(0) as u32;
                        current_viewport = Rect { x, y, width: w, height: h };
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

                let stack_trace = cmd.stack_trace();

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

                let stack_trace = cmd.stack_trace();

                let sample_key = {
                    let mut h = rustc_hash::FxHasher::default();
                    target.hash(&mut h);
                    offset.hash(&mut h);
                    sample_str.as_deref().unwrap_or("").hash(&mut h);
                    h.finish()
                };
                let is_duplicate = seen_buffer_samples.contains(&sample_key);
                seen_buffer_samples.insert(sample_key);
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
            name if name.starts_with("uniform") => {
                summary.uniform_calls += 1;
                let mut loc_id: Option<u32> = None;
                let mut is_redundant = false;
                let mut bytes = 4;

                if name.contains("Matrix4") {
                    bytes = 64;
                } else if name.contains("Matrix3") {
                    bytes = 36;
                } else if name.contains("Matrix2") || name.ends_with("4f") || name.ends_with("4i") || name.ends_with("4ui") || name.ends_with("4fv") || name.ends_with("4iv") {
                    bytes = 16;
                } else if name.ends_with("3f") || name.ends_with("3i") || name.ends_with("3ui") || name.ends_with("3fv") || name.ends_with("3iv") {
                    bytes = 12;
                } else if name.ends_with("2f") || name.ends_with("2i") || name.ends_with("2ui") || name.ends_with("2fv") || name.ends_with("2iv") {
                    bytes = 8;
                }

                if let Some(args) = args {
                    if let Some(first) = args.first() {
                        loc_id = extract_spektor_id(first);
                    }
                    val_buf.clear();
                    if args.len() >= 2 {
                        use std::fmt::Write as _;
                        let _ = write!(val_buf, "{}", args[1]);
                    }
                } else {
                    val_buf.clear();
                }
                let fp = {
                    let mut h = rustc_hash::FxHasher::default();
                    val_buf.as_bytes().hash(&mut h);
                    h.finish()
                };

                if let (Some(pid), Some(lid)) = (current_program_id, loc_id) {
                    let ukey = (pid, lid);
                    if let Some(prev) = current_uniform_values.get(&ukey) {
                        if *prev == fp {
                            is_redundant = true;
                            summary.redundant_uniform_calls += 1;
                            summary.redundant_uniform_bytes += bytes;
                        }
                    }
                    current_uniform_values.insert(ukey, fp);
                }
                summary.uniform_bytes += bytes;

                let stack_caller = cmd.stack_trace.as_ref().and_then(|arr| arr.first().map(|s| s.to_string()));

                uniform_commands.push(UniformCommandItem {
                    command_index: cmd_idx,
                    name: name.to_string(),
                    location_id: loc_id,
                    bytes,
                    is_redundant,
                    stack_caller,
                });
            }
            _ => {}
        }

        // Check if it's a draw command
        let is_draw = name.starts_with("draw");
        if is_draw {
            summary.draw_calls += 1;
            let dc = cmd.draw_call.as_ref();

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
                if let Some(shaders) = &dc_obj.shaders {
                    if let Some(vs) = shaders.first().and_then(|s| s.source.as_deref()) {
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
            let mut uniform_count = 0;
            let mut samplers = Vec::new();
            if let Some(dc_obj) = dc {
                if let Some(uniforms) = &dc_obj.uniforms {
                    uniform_count = uniforms.len();
                    for u in uniforms {
                        let uname = u.name.unwrap_or("");
                        let utype = u.utype.unwrap_or("");
                        if utype.contains("SAMPLER") {
                            samplers.push(uname.to_string());
                        }
                        if uname == "glowColor" {
                            if let Some(arr) = u.value.as_ref().and_then(|v| v.as_array()) {
                                if arr.len() >= 3 {
                                    let r = arr[0].as_f64().unwrap_or(0.0) as f32;
                                    let g = arr[1].as_f64().unwrap_or(0.0) as f32;
                                    let b = arr[2].as_f64().unwrap_or(0.0) as f32;
                                    let a = arr.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                                    glow_color = Some([r, g, b, a]);
                                }
                            }
                        } else if uname == "world" {
                            if let Some(arr) = u.value.as_ref().and_then(|v| v.as_array()) {
                                if arr.len() >= 16 {
                                    let x = arr[12].as_f64().unwrap_or(0.0) as f32;
                                    let y = arr[13].as_f64().unwrap_or(0.0) as f32;
                                    let z = arr[14].as_f64().unwrap_or(0.0) as f32;
                                    world_position = Some([x, y, z]);
                                }
                            }
                        }

                        if uname.to_lowercase().contains("emissive") && u.value.as_ref().map(|v| v.is_object()).unwrap_or(false) {
                            has_emissive_tex = true;
                        }

                        // Check samplers mapping to texture units
                        if let Some(val) = &u.value {
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
                if let Some(attrs) = &dc_obj.attributes {
                    for attr in attrs {
                        let name = attr.name.unwrap_or("");
                        let blen = attr.buffer_length.as_ref().and_then(|v| v.as_u64()).unwrap_or(0) as usize;
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

            let stack_component = cmd.stack_trace.as_ref().and_then(|arr| {
                for f in arr {
                    if f.contains("ThinGlowLayer") {
                        return Some("ThinGlowLayer".to_string());
                    } else if f.contains("Mesh._") || f.contains("Mesh.") {
                        return Some("Mesh".to_string());
                    } else if f.contains("Particle") {
                        return Some("ParticleSystem".to_string());
                    } else if f.contains("Procedural") {
                        return Some("ProceduralTexture".to_string());
                    } else if f.contains("Scene.") {
                        return Some("Scene".to_string());
                    }
                }
                arr.first().map(|s| s.split('(').next().unwrap_or(s).trim().to_string())
            });

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
                pass_index: passes.len(),
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
                uniform_count,
                active_uniform_count: uniform_count,
                samplers,
                stack_component,
                viewport: current_viewport,
                scissor: current_scissor,
                drawing_buffer,
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
        passes.push(current_pass);
    }

    // Set pass types for all passes
    for p in &mut passes {
        p.pass_type = classify_pass(&p.fbo_name, &p.shaders);
    }

    let (mut textures, total_texture_bytes) = compile_textures(raw_textures, &tex_draw_counts, &tex_bound_units, &tex_uniform_names);
    textures.sort_by_key(|a| std::cmp::Reverse(a.length));
    summary.total_texture_bytes = total_texture_bytes;
    summary.total_textures_count = textures.len();
    summary.unique_programs = program_draw_counts.len();

    let programs = compile_programs(program_draw_counts, program_shaders);

    summary.particle_systems_count = particle_systems.len();
    summary.total_particles = particle_systems.iter().map(|p| p.particle_count).sum();

    // Count active FBOs
    let active_fbos: FxHashSet<&str> = passes.iter().map(|p| p.fbo_name.as_str()).collect();
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
    if let Some(mem) = root.memory.as_ref() {
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
        uniform_commands,
    })
}
