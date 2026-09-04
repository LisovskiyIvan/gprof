use crate::model::ParsedCapture;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverdrawAnalysisResult {
    pub total_screen_pixels: usize,
    pub backbuffer_resolution: (u32, u32),
    pub total_shaded_pixels_estimate: usize,
    pub overdraw_factor: f64,
    pub fullscreen_quad_passes: usize,
    pub blended_draw_calls: usize,
    pub blended_triangles: usize,
    pub early_z_broken_draws: usize,
    pub pass_overdraw: Vec<PassOverdrawItem>,
    pub risk_level: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassOverdrawItem {
    pub pass_index: usize,
    pub fbo_name: String,
    pub resolution: (u32, u32),
    pub draw_count: usize,
    pub quad_draws: usize,
    pub opaque_draws: usize,
    pub blended_draws: usize,
    pub blended_triangles: usize,
    pub estimated_fillrate_multiplier: f64,
    pub estimated_shaded_pixels: usize,
    pub fillrate_pressure: String,
}

pub fn analyze_overdraw(capture: &ParsedCapture) -> OverdrawAnalysisResult {
    // Determine backbuffer resolution
    let mut bb_width = capture.canvas.as_ref().and_then(|c| c.width).unwrap_or(0);
    let mut bb_height = capture.canvas.as_ref().and_then(|c| c.height).unwrap_or(0);

    if bb_width == 0 || bb_height == 0 {
        for p in &capture.passes {
            if p.fbo_name.to_lowercase().contains("backbuffer") {
                if let (Some(w), Some(h)) = (p.viewport_width, p.viewport_height) {
                    if w > 0 && h > 0 {
                        bb_width = w;
                        bb_height = h;
                        break;
                    }
                }
            }
        }
    }

    if bb_width == 0 || bb_height == 0 {
        // Fallback to highest pass viewport
        for p in &capture.passes {
            if let (Some(w), Some(h)) = (p.viewport_width, p.viewport_height) {
                if w > bb_width {
                    bb_width = w;
                    bb_height = h;
                }
            }
        }
    }

    if bb_width == 0 || bb_height == 0 {
        bb_width = 1920;
        bb_height = 1080;
    }

    let backbuffer_resolution = (bb_width, bb_height);
    let total_screen_pixels = (bb_width as usize) * (bb_height as usize);

    let mut pass_overdraw = Vec::new();
    let mut total_shaded_pixels_estimate = 0;
    let mut fullscreen_quad_passes = 0;
    let mut total_blended_draws = 0;
    let mut total_blended_triangles = 0;
    let mut early_z_broken_draws = 0;

    let mut draw_cursor = 0;
    for (p_idx, pass) in capture.passes.iter().enumerate() {
        let p_draw_count = pass.draw_count;
        let pass_draws = if draw_cursor + p_draw_count <= capture.draws.len() {
            &capture.draws[draw_cursor..draw_cursor + p_draw_count]
        } else {
            &capture.draws[draw_cursor..]
        };
        draw_cursor += p_draw_count;

        let w = pass.viewport_width.unwrap_or(bb_width);
        let h = pass.viewport_height.unwrap_or(bb_height);
        let pass_pixels = (w as usize) * (h as usize);

        let mut quad_draws = 0;
        let mut opaque_draws = 0;
        let mut blended_draws = 0;
        let mut blended_tris = 0;
        let mut seen_blend = false;
        let mut pass_early_z_broken = 0;

        for d in pass_draws {
            let is_quad = d.mode == 4 && d.triangles == 2;
            if is_quad {
                quad_draws += 1;
            }

            if d.blend_enabled {
                blended_draws += 1;
                blended_tris += d.triangles;
                seen_blend = true;
            } else {
                opaque_draws += 1;
                if seen_blend {
                    pass_early_z_broken += 1;
                }
            }
        }

        total_blended_draws += blended_draws;
        total_blended_triangles += blended_tris;
        early_z_broken_draws += pass_early_z_broken;

        if quad_draws > 0 {
            fullscreen_quad_passes += 1;
        }

        // Calculate fillrate multiplier for this pass
        // 1. Quads: each quad shades 100% of viewport
        let quad_coverage = quad_draws as f64 * 1.0;
        // 2. Opaque geometry: base ~1.0x with Early-Z, plus 0.5x per early-z broken draw
        let opaque_coverage = if opaque_draws > quad_draws {
            1.0 + (pass_early_z_broken as f64 * 0.05).min(2.0)
        } else {
            0.0
        };
        // 3. Blended geometry: each blended draw adds ~0.1x to 0.5x coverage
        let blended_coverage = (blended_draws as f64 * 0.15).min(4.0);

        let multiplier = (quad_coverage + opaque_coverage + blended_coverage).max(if pass_draws.is_empty() { 0.0 } else { 0.1 });
        let shaded_pixels = (pass_pixels as f64 * multiplier) as usize;
        total_shaded_pixels_estimate += shaded_pixels;

        let pressure = if multiplier >= 3.0 {
            "🔴 Critical".to_string()
        } else if multiplier >= 1.8 {
            "🟠 High".to_string()
        } else if multiplier >= 1.0 {
            "🟡 Moderate".to_string()
        } else {
            "🟢 Low".to_string()
        };

        pass_overdraw.push(PassOverdrawItem {
            pass_index: p_idx,
            fbo_name: pass.fbo_name.clone(),
            resolution: (w, h),
            draw_count: p_draw_count,
            quad_draws,
            opaque_draws,
            blended_draws,
            blended_triangles: blended_tris,
            estimated_fillrate_multiplier: multiplier,
            estimated_shaded_pixels: shaded_pixels,
            fillrate_pressure: pressure,
        });
    }

    let overdraw_factor = if total_screen_pixels > 0 {
        total_shaded_pixels_estimate as f64 / total_screen_pixels as f64
    } else {
        0.0
    };

    let risk_level = if overdraw_factor >= 5.0 {
        "Critical".to_string()
    } else if overdraw_factor >= 3.5 {
        "High".to_string()
    } else if overdraw_factor >= 2.0 {
        "Moderate".to_string()
    } else {
        "Low".to_string()
    };

    let mut recommendations = Vec::new();
    if overdraw_factor >= 3.0 {
        recommendations.push(format!(
            "⚠️ High Overdraw Pressure: Frame shades an estimated {:.1}x screen pixel equivalents ({} shaded pixels vs {} native). On mobile GPUs, this triggers thermal throttling and drops FPS.",
            overdraw_factor, total_shaded_pixels_estimate, total_screen_pixels
        ));
    }

    if fullscreen_quad_passes >= 3 {
        recommendations.push(format!(
            "⚠️ {} full-screen post-processing passes detected. Ensure blur and downsample FBOs run at 0.5x or 0.25x viewport resolution instead of full screen resolution.",
            fullscreen_quad_passes
        ));
    }

    if early_z_broken_draws > 0 {
        recommendations.push(format!(
            "⚠️ {} opaque draw calls were submitted after alpha-blended draws, bypassing GPU Early-Z depth rejection and causing redundant pixel shading.",
            early_z_broken_draws
        ));
    }

    if total_blended_draws > 25 {
        recommendations.push(format!(
            "ℹ️ {} blended draws rendered {} alpha triangles. Check GPU particle emitters and soft alpha effects for dense overlapping billboards.",
            total_blended_draws, total_blended_triangles
        ));
    }

    OverdrawAnalysisResult {
        total_screen_pixels,
        backbuffer_resolution,
        total_shaded_pixels_estimate,
        overdraw_factor,
        fullscreen_quad_passes,
        blended_draw_calls: total_blended_draws,
        blended_triangles: total_blended_triangles,
        early_z_broken_draws,
        pass_overdraw,
        risk_level,
        recommendations,
    }
}
