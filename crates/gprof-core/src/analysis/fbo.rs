use crate::model::RenderPassItem;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FboSummaryItem {
    pub fbo_name: String,
    pub width: u32,
    pub height: u32,
    pub vram_bytes: usize,
    pub pass_count: usize,
    pub draw_count: usize,
    pub triangles: usize,
    pub pass_types: Vec<String>,
    pub is_oversized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FboAnalysis {
    pub total_fbos_count: usize,
    pub total_offscreen_vram_bytes: usize,
    pub backbuffer_resolution: Option<(u32, u32)>,
    pub fbos: Vec<FboSummaryItem>,
    pub warnings: Vec<String>,
}

pub fn analyze_fbos(passes: &[RenderPassItem]) -> FboAnalysis {
    let mut backbuffer_res = None;

    for p in passes {
        if p.fbo_name.contains("Default") || p.fbo_name.contains("Backbuffer") {
            if let (Some(w), Some(h)) = (p.viewport_width, p.viewport_height) {
                if w > 0 && h > 0 {
                    backbuffer_res = Some((w, h));
                    break;
                }
            }
        }
    }

    let mut map: FxHashMap<String, FboSummaryItem> = FxHashMap::default();

    for p in passes {
        let entry = map.entry(p.fbo_name.clone()).or_insert_with(|| FboSummaryItem {
            fbo_name: p.fbo_name.clone(),
            width: p.viewport_width.unwrap_or(0),
            height: p.viewport_height.unwrap_or(0),
            vram_bytes: p.vram_bytes,
            pass_count: 0,
            draw_count: 0,
            triangles: 0,
            pass_types: Vec::new(),
            is_oversized: false,
        });

        if entry.width == 0 && p.viewport_width.is_some() {
            entry.width = p.viewport_width.unwrap_or(0);
            entry.height = p.viewport_height.unwrap_or(0);
            entry.vram_bytes = p.vram_bytes;
        }

        entry.pass_count += 1;
        entry.draw_count += p.draw_count;
        entry.triangles += p.triangles;
        if !entry.pass_types.contains(&p.pass_type) {
            entry.pass_types.push(p.pass_type.clone());
        }
    }

    let mut fbos: Vec<FboSummaryItem> = map.into_values().collect();
    fbos.sort_by(|a, b| b.vram_bytes.cmp(&a.vram_bytes));

    let mut total_offscreen_vram = 0;
    let mut warnings = Vec::new();

    for f in &mut fbos {
        let is_backbuffer = f.fbo_name.contains("Default") || f.fbo_name.contains("Backbuffer");
        if !is_backbuffer {
            total_offscreen_vram += f.vram_bytes;

            if let Some((bw, bh)) = backbuffer_res {
                let is_postprocess = f.pass_types.iter().any(|t| t.contains("Glow") || t.contains("Blur"));
                if is_postprocess && f.width >= bw && f.height >= bh && bw > 0 {
                    f.is_oversized = true;
                    warnings.push(format!(
                        "FBO '{}' (used for post-processing) is running at 100% full screen resolution ({}x{}). Downscaling to half-resolution (0.5x) saves 75% fillrate.",
                        f.fbo_name, f.width, f.height
                    ));
                }
            }
        }
    }

    FboAnalysis {
        total_fbos_count: fbos.len(),
        total_offscreen_vram_bytes: total_offscreen_vram,
        backbuffer_resolution: backbuffer_res,
        fbos,
        warnings,
    }
}
