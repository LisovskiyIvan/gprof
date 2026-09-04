use crate::model::*;
use crate::report::{format_bytes, format_num};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDelta {
    pub name: String,
    pub before: f64,
    pub after: f64,
    pub delta: f64,
    pub pct_change: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareResult {
    pub metrics: Vec<MetricDelta>,
    pub textures_added: usize,
    pub textures_removed: usize,
    pub vram_delta_bytes: i64,
    pub glow_triangles_saved: usize,
}

pub fn compare_captures(before: &ParsedCapture, after: &ParsedCapture) -> CompareResult {
    let mut metrics = Vec::new();

    let add_metric = |list: &mut Vec<MetricDelta>, name: &str, b: usize, a: usize, unit: &str| {
        let b_f = b as f64;
        let a_f = a as f64;
        let delta = a_f - b_f;
        let pct = if b_f > 0.0 { (delta / b_f) * 100.0 } else { 0.0 };
        list.push(MetricDelta {
            name: name.to_string(),
            before: b_f,
            after: a_f,
            delta,
            pct_change: pct,
            unit: unit.to_string(),
        });
    };

    add_metric(&mut metrics, "Draw Calls", before.summary.draw_calls, after.summary.draw_calls, "calls");
    add_metric(&mut metrics, "Triangles", before.summary.triangles, after.summary.triangles, "tris");
    add_metric(&mut metrics, "Points / Particles", before.summary.points, after.summary.points, "points");
    add_metric(&mut metrics, "Texture Memory", before.summary.total_texture_bytes / (1024 * 1024), after.summary.total_texture_bytes / (1024 * 1024), "MB");
    add_metric(&mut metrics, "Glow Total Draws", before.summary.glow_total_draws, after.summary.glow_total_draws, "draws");
    add_metric(&mut metrics, "Glow Black Occluder Draws", before.summary.glow_black_draws, after.summary.glow_black_draws, "draws");
    add_metric(&mut metrics, "Glow Occluder Triangles", before.summary.glow_black_triangles, after.summary.glow_black_triangles, "tris");
    add_metric(&mut metrics, "Programs Used", before.summary.unique_programs, after.summary.unique_programs, "programs");
    add_metric(&mut metrics, "Program Switches (useProgram)", before.summary.use_program_calls, after.summary.use_program_calls, "switches");
    add_metric(&mut metrics, "Redundant State Calls", before.state_changes.total_redundant_calls, after.state_changes.total_redundant_calls, "calls");
    add_metric(&mut metrics, "32-Bit Index Mesh Draws", before.summary.index_32bit_draws, after.summary.index_32bit_draws, "draws");
    add_metric(&mut metrics, "Dynamic Texture Uploads (texImage2D)", before.summary.dynamic_uploads_count, after.summary.dynamic_uploads_count, "uploads");
    add_metric(&mut metrics, "Dynamic Buffer Uploads (bufferSubData)", before.summary.buffer_subdata_calls, after.summary.buffer_subdata_calls, "uploads");
    add_metric(&mut metrics, "Duplicate Buffer Uploads", before.summary.duplicate_buffer_uploads, after.summary.duplicate_buffer_uploads, "uploads");

    let vram_delta = after.summary.total_texture_bytes as i64 - before.summary.total_texture_bytes as i64;
    let glow_saved = before.summary.glow_black_triangles.saturating_sub(after.summary.glow_black_triangles);

    CompareResult {
        metrics,
        textures_added: after.textures.len().saturating_sub(before.textures.len()),
        textures_removed: before.textures.len().saturating_sub(after.textures.len()),
        vram_delta_bytes: vram_delta,
        glow_triangles_saved: glow_saved,
    }
}

pub fn format_compare_markdown(before: &ParsedCapture, after: &ParsedCapture, res: &CompareResult) -> String {
    let mut out = String::new();
    out.push_str("# gprof — Capture Comparison (Before vs After)\n\n");

    out.push_str("## Executive Comparison Summary\n\n");
    out.push_str("| Metric | Before | After | Delta | Delta % |\n");
    out.push_str("|---|---|---|---|---|\n");

    for m in &res.metrics {
        let sign = if m.delta > 0.0 { "+" } else { "" };
        let (b_str, a_str, d_str) = if m.unit == "MB" {
            (format!("{:.1} MB", m.before), format!("{:.1} MB", m.after), format!("{}{:.1} MB", sign, m.delta))
        } else {
            (format_num(m.before as usize), format_num(m.after as usize), format!("{}{}", sign, format_num(m.delta.abs() as usize)))
        };
        let pct_str = format!("{}{:.1}%", sign, m.pct_change);
        out.push_str(&format!("| **{}** | {} | {} | **{}** | **{}** |\n", m.name, b_str, a_str, d_str, pct_str));
    }
    out.push_str("\n");

    out.push_str("## Key Observations\n\n");
    if res.glow_triangles_saved > 0 {
        out.push_str(&format!("- 🟢 **GlowLayer Occluders:** Saved **{}** triangles ({:.1}%) of wasted occluder geometry.\n", format_num(res.glow_triangles_saved), (res.glow_triangles_saved as f64 / before.summary.glow_total_triangles as f64) * 100.0));
    }
    if res.vram_delta_bytes < 0 {
        out.push_str(&format!("- 🟢 **Texture Memory:** Freed **{}** of VRAM.\n", format_bytes(res.vram_delta_bytes.unsigned_abs() as usize)));
    } else if res.vram_delta_bytes > 0 {
        out.push_str(&format!("- 🔴 **Texture Memory:** Increased by **{}** of VRAM.\n", format_bytes(res.vram_delta_bytes as usize)));
    }
    let dc_delta = after.summary.draw_calls as i64 - before.summary.draw_calls as i64;
    if dc_delta < 0 {
        out.push_str(&format!("- 🟢 **Draw Calls:** Reduced by **{}** draw calls.\n", dc_delta.unsigned_abs()));
    } else if dc_delta > 0 {
        out.push_str(&format!("- 🔴 **Draw Calls:** Increased by **{}** draw calls.\n", dc_delta));
    }
    out.push_str("\n");

    out
}
