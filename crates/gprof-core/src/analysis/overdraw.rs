use crate::model::ParsedCapture;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverdrawAnalysisResult {
    pub total_screen_pixels: usize,
    pub backbuffer_resolution: (u32, u32),
    pub total_shaded_pixels_estimate: usize,
    pub overdraw_factor: f64,
    pub estimated_screenspace_coverage: f64,
    pub postprocess_quad_coverage: f64,
    pub geometry_coverage: f64,
    pub fullscreen_quad_passes: usize,
    pub blended_draw_calls: usize,
    pub blended_triangles: usize,
    pub early_z_broken_draws: usize,
    pub pass_overdraw: Vec<PassOverdrawItem>,
    pub risk_level: String,
    pub methodology_disclaimer: String,
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
    pub quad_multiplier: f64,
    pub geometry_multiplier: f64,
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
            if p.drawing_buffer.0 > 0 && p.drawing_buffer.1 > 0 {
                bb_width = p.drawing_buffer.0;
                bb_height = p.drawing_buffer.1;
                break;
            }
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
    let mut total_quad_shaded_pixels = 0;
    let mut total_geometry_shaded_pixels = 0;
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

        let mut quad_draws: usize = 0;
        let mut opaque_draws: usize = 0;
        let mut blended_draws: usize = 0;
        let mut blended_tris: usize = 0;
        let mut seen_blend = false;
        let mut pass_early_z_broken: usize = 0;

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

        // Calculate fillrate multipliers:
        // 1. Quads: each quad covers 100% of the viewport
        let quad_multiplier = quad_draws as f64 * 1.0;
        // 2. Geometry: opaque base ~1.0x with Early-Z + blended additions
        let opaque_geo_draws = opaque_draws.saturating_sub(quad_draws);
        let opaque_coverage = if opaque_geo_draws > 0 { 1.0 + (pass_early_z_broken as f64 * 0.05).min(2.0) } else { 0.0 };
        let blended_coverage = (blended_draws as f64 * 0.15).min(4.0);
        let geometry_multiplier = opaque_coverage + blended_coverage;

        let multiplier = (quad_multiplier + geometry_multiplier).max(if pass_draws.is_empty() { 0.0 } else { 0.1 });
        let quad_shaded = (pass_pixels as f64 * quad_multiplier) as usize;
        let geo_shaded = (pass_pixels as f64 * geometry_multiplier) as usize;
        let shaded_pixels = (pass_pixels as f64 * multiplier) as usize;

        total_quad_shaded_pixels += quad_shaded;
        total_geometry_shaded_pixels += geo_shaded;
        total_shaded_pixels_estimate += shaded_pixels;

        let pressure = if multiplier >= 3.0 {
            "High Coverage".to_string()
        } else if multiplier >= 1.8 {
            "Moderate-High".to_string()
        } else if multiplier >= 1.0 {
            "Moderate".to_string()
        } else {
            "Low".to_string()
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
            quad_multiplier,
            geometry_multiplier,
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

    let postprocess_quad_coverage = if total_screen_pixels > 0 {
        total_quad_shaded_pixels as f64 / total_screen_pixels as f64
    } else {
        0.0
    };

    let geometry_coverage = if total_screen_pixels > 0 {
        total_geometry_shaded_pixels as f64 / total_screen_pixels as f64
    } else {
        0.0
    };

    let estimated_screenspace_coverage = overdraw_factor;

    let risk_level = if overdraw_factor >= 6.0 {
        "Elevated Screen Coverage (High)".to_string()
    } else if overdraw_factor >= 3.5 {
        "Moderate-High Screen Coverage".to_string()
    } else if overdraw_factor >= 2.0 {
        "Moderate Screen Coverage".to_string()
    } else {
        "Low Screen Coverage".to_string()
    };

    let methodology_disclaimer = "Heuristic estimate based on pass types, full-screen post-processing quads, and draw call counts. Does not model hardware Early-Z depth rejection, shader discard, occlusion culling, or exact mesh screen-space bounds. Without GPU fragment execution counters/timestamps, this represents screen coverage pressure rather than measured pixel fill bottleneck.".to_string();

    let mut recommendations = Vec::new();
    if overdraw_factor >= 3.0 {
        recommendations.push(format!(
            "[Calculated Estimate] Frame has an estimated {:.1}x screen-space coverage multiplier (approx. {:.1}x from post-process quads, {:.1}x from scene geometry & alpha blending; {} shaded pixel equivalents vs {} native). On mobile or tile-based GPUs, high coverage increases memory bandwidth.",
            overdraw_factor, postprocess_quad_coverage, geometry_coverage, total_shaded_pixels_estimate, total_screen_pixels
        ));
    }

    if fullscreen_quad_passes >= 3 {
        recommendations.push(format!(
            "[Hypothesis / Candidate] {} full-screen post-processing passes detected ({:.1}x coverage). Verify whether blur, bloom, and glow FBOs can run at half (0.5x) or quarter (0.25x) resolution instead of 1.0x.",
            fullscreen_quad_passes, postprocess_quad_coverage
        ));
    }

    if early_z_broken_draws > 0 {
        recommendations.push(format!(
            "[Hypothesis / Candidate] {} opaque draw calls were submitted after alpha-blended draws within passes. If rendering 3D scene geometry after transparent meshes, reorder opaque objects first to maximize hardware Early-Z depth rejection (ignore if these are 2D HUD/overlay passes).",
            early_z_broken_draws
        ));
    }

    if total_blended_draws > 25 {
        recommendations.push(format!(
            "[Observed Fact] {} blended draws rendered {} alpha triangles. Check GPU particle emitters and transparent billboards for dense multi-layer overlap.",
            total_blended_draws, total_blended_triangles
        ));
    }

    OverdrawAnalysisResult {
        total_screen_pixels,
        backbuffer_resolution,
        total_shaded_pixels_estimate,
        overdraw_factor,
        estimated_screenspace_coverage,
        postprocess_quad_coverage,
        geometry_coverage,
        fullscreen_quad_passes,
        blended_draw_calls: total_blended_draws,
        blended_triangles: total_blended_triangles,
        early_z_broken_draws,
        pass_overdraw,
        risk_level,
        methodology_disclaimer,
        recommendations,
    }
}
