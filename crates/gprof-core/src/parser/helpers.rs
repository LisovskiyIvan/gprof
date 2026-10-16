use crate::model::{CompressionStatus, ProgramItem, TextureItem};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

use crate::gl_enum;
use crate::model::TextureUsageCategory;

#[derive(Default, Clone)]
pub(super) struct RawTexData {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) format: u32,
    pub(super) internal_format: u32,
    pub(super) r#type: u32,
    pub(super) length: usize,
    pub(super) is_compressed: bool,
    pub(super) target: String,
}

pub(super) fn classify_pass(fbo: &str, shaders: &[String]) -> String {
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

pub(super) fn extract_spektor_id(v: &Value) -> Option<u32> {
    if let Some(tag) = v.get("__SPECTOR_Object_TAG") {
        tag.get("id").and_then(|id| id.as_u64()).map(|id| id as u32)
    } else {
        None
    }
}

pub(super) fn classify_texture_category(uniform_names: &[String], internal_format: u32, format: u32, type_val: u32, width: u32, height: u32, target: &str) -> TextureUsageCategory {
    let unames_lower: Vec<String> = uniform_names.iter().map(|u| u.to_lowercase()).collect();
    let is_anim = unames_lower.iter().any(|u| {
        u.contains("vat")
            || u.contains("bone")
            || u.contains("palette")
            || u.contains("additive")
            || u.contains("motion")
            || u.contains("velocity")
            || u.contains("morph")
            || u.contains("upperadditive")
            || u.contains("upperwholebody")
            || u.contains("random")
            || u.contains("noise")
            || u.contains("lut")
            || u.contains("curve")
            || u.contains("ramp")
            || u.contains("data")
    });
    if is_anim {
        return TextureUsageCategory::GpuAnimationData;
    }

    // 1D buffer textures (e.g. 16384x1 or 1x16384) are data/LUT textures
    if (width > 0 && height > 0) && (width == 1 || height == 1) {
        return TextureUsageCategory::GpuAnimationData;
    }

    let is_rt = unames_lower
        .iter()
        .any(|u| u.contains("blur") || u.contains("kernel") || u.contains("merge") || u.contains("depth") || u.contains("shadow") || u.contains("fbo") || u.contains("pass") || u.contains("rtt"));
    if is_rt {
        return TextureUsageCategory::RenderTarget;
    }

    let is_color = unames_lower.iter().any(|u| {
        u.contains("diffuse")
            || u.contains("albedo")
            || u.contains("color")
            || u.contains("base")
            || u.contains("bump")
            || u.contains("normal")
            || u.contains("specular")
            || u.contains("emissive")
            || u.contains("reflectivity")
            || u.contains("mask")
            || u.contains("star")
            || u.contains("sky")
            || u.contains("lightmap")
            || u.contains("ambient")
            || u.contains("opacity")
    });
    if is_color {
        return TextureUsageCategory::ColorMaterial;
    }

    // Format-based heuristic: Float / Half-Float without explicit color sampler names is data
    if gl_enum::is_float_or_half_float(internal_format, format, type_val) {
        return TextureUsageCategory::GpuAnimationData;
    }

    // Extreme aspect ratio (e.g. Nx1 or 1xN or > 8:1 ratio with height > 2000)
    if (width > 0 && height > 0) && ((height > width * 8) || (width > height * 8)) {
        return TextureUsageCategory::GpuAnimationData;
    }

    if target == "TEXTURE_CUBE_MAP" {
        return TextureUsageCategory::ColorMaterial;
    }

    if internal_format != 0 || format != 0 {
        return TextureUsageCategory::ColorMaterial;
    }

    TextureUsageCategory::Unknown
}

pub(super) fn collect_textures(v: &Value, map: &mut FxHashMap<u32, RawTexData>) {
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
                            item.is_compressed =
                                cd.get("isCompressed").and_then(|c| c.as_bool()).unwrap_or(false) || gl_enum::is_compressed_format(item.internal_format) || gl_enum::is_compressed_format(item.format);
                            item.target = cd.get("target").and_then(|t| t.as_str()).unwrap_or("").to_string();
                        }
                        map.entry(id)
                            .and_modify(|existing| {
                                if item.length > existing.length || (existing.internal_format == 0 && item.internal_format != 0) {
                                    *existing = item.clone();
                                }
                            })
                            .or_insert(item);
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

pub(super) fn compile_textures(
    raw_textures: FxHashMap<u32, RawTexData>,
    tex_draw_counts: &FxHashMap<u32, usize>,
    tex_bound_units: &FxHashMap<u32, FxHashSet<u32>>,
    tex_uniform_names: &FxHashMap<u32, FxHashSet<String>>,
) -> (Vec<TextureItem>, usize) {
    let mut textures = Vec::with_capacity(raw_textures.len());
    let mut total_texture_bytes = 0;
    for (tid, raw) in raw_textures {
        let draw_cnt = tex_draw_counts.get(&tid).copied().unwrap_or(0);
        let units: Vec<u32> = tex_bound_units.get(&tid).map(|s| s.iter().copied().collect()).unwrap_or_default();
        let unames: Vec<String> = tex_uniform_names.get(&tid).map(|s| s.iter().cloned().collect()).unwrap_or_default();
        let is_pot = raw.width > 0 && raw.height > 0 && raw.width.is_power_of_two() && raw.height.is_power_of_two();

        let (vram_bytes, base_bytes, mip_levels, comp_status, method) = gl_enum::calculate_texture_memory(raw.width, raw.height, raw.internal_format, raw.format, raw.r#type, &raw.target, raw.length);
        let category = classify_texture_category(&unames, raw.internal_format, raw.format, raw.r#type, raw.width, raw.height, &raw.target);
        total_texture_bytes += vram_bytes;
        let is_compressed = comp_status == CompressionStatus::Compressed;

        let format_display = if raw.format == 0 {
            if is_compressed { "—".to_string() } else { "UNKNOWN".to_string() }
        } else {
            gl_enum::format_name(raw.format).to_string()
        };
        let type_display = if raw.r#type == 0 {
            if is_compressed { "—".to_string() } else { "UNKNOWN".to_string() }
        } else {
            gl_enum::type_name(raw.r#type).to_string()
        };

        textures.push(TextureItem {
            id: tid,
            width: raw.width,
            height: raw.height,
            format: raw.format,
            format_name: format_display,
            internal_format: raw.internal_format,
            internal_format_name: gl_enum::format_name(raw.internal_format).to_string(),
            r#type: raw.r#type,
            type_name: type_display,
            length: vram_bytes,
            base_bytes,
            mip_levels,
            is_compressed,
            compression_status: comp_status,
            estimation_method: method,
            category,
            target: raw.target,
            draw_count: draw_cnt,
            bound_units: units,
            uniform_names: unames,
            is_pot,
        });
    }
    (textures, total_texture_bytes)
}

pub(super) fn compile_programs(
    program_draw_counts: FxHashMap<u32, usize>,
    mut program_shaders: FxHashMap<u32, (String, Vec<String>)>,
) -> Vec<ProgramItem> {
    let mut programs = Vec::with_capacity(program_draw_counts.len());
    for (pid, cnt) in program_draw_counts {
        let (sname, defs) = program_shaders.remove(&pid).unwrap_or_else(|| ("unknown".to_string(), Vec::new()));
        let is_pbr = sname.contains("pbr");
        programs.push(ProgramItem { id: pid, draw_count: cnt, shader_name: sname, defines: defs, is_pbr });
    }
    programs.sort_by_key(|a| std::cmp::Reverse(a.draw_count));
    programs
}
