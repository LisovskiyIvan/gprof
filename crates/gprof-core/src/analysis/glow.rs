use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlowOccluderMesh {
    pub draw_index: usize,
    pub command_id: usize,
    pub triangles: usize,
    pub world_position: Option<[f32; 3]>,
    pub vertex_buffer_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlowAuditResult {
    pub total_draws: usize,
    pub total_triangles: usize,
    pub black_occluder_draws: usize,
    pub black_occluder_triangles: usize,
    pub active_glowing_draws: usize,
    pub active_glowing_triangles: usize,
    pub wasted_triangle_percentage: f64,
    pub top_black_occluders: Vec<GlowOccluderMesh>,
    pub recommendation: String,
}

pub fn audit_glow_layer(draws: &[DrawCallItem]) -> GlowAuditResult {
    let glow_draws: Vec<&DrawCallItem> = draws.iter().filter(|d| d.is_glow_pass).collect();
    let total_draws = glow_draws.len();
    let total_triangles: usize = glow_draws.iter().map(|d| d.triangles).sum();

    let black_occluders: Vec<&DrawCallItem> = glow_draws.iter().filter(|d| d.is_black_glow).copied().collect();
    let black_draws = black_occluders.len();
    let black_triangles: usize = black_occluders.iter().map(|d| d.triangles).sum();

    let active_draws = total_draws.saturating_sub(black_draws);
    let active_triangles = total_triangles.saturating_sub(black_triangles);

    let wasted_pct = if total_triangles > 0 {
        (black_triangles as f64 / total_triangles as f64) * 100.0
    } else {
        0.0
    };

    let mut occluders_sorted = black_occluders.clone();
    occluders_sorted.sort_by(|a, b| b.triangles.cmp(&a.triangles));

    let top_occluders = occluders_sorted.into_iter().take(15).map(|d| GlowOccluderMesh {
        draw_index: d.index,
        command_id: d.command_id,
        triangles: d.triangles,
        world_position: d.world_position,
        vertex_buffer_bytes: d.vertex_buffer_bytes,
    }).collect();

    let rec = if black_draws > 0 {
        format!(
            "SWITCH TO WHITELIST: {} of {} draw calls ({:.1}% triangles) in GlowLayer are pure black occluders. Use `glowLayer.addIncludedOnlyMesh()` to render only the {} glowing meshes.",
            black_draws,
            total_draws,
            wasted_pct,
            active_draws
        )
    } else {
        "GlowLayer is optimal: no black occluders detected.".to_string()
    };

    GlowAuditResult {
        total_draws,
        total_triangles,
        black_occluder_draws: black_draws,
        black_occluder_triangles: black_triangles,
        active_glowing_draws: active_draws,
        active_glowing_triangles: active_triangles,
        wasted_triangle_percentage: wasted_pct,
        top_black_occluders: top_occluders,
        recommendation: rec,
    }
}
