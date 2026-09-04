use crate::model::ParsedCapture;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassSortingItem {
    pub pass_index: usize,
    pub fbo_name: String,
    pub draw_count: usize,
    pub shader_switches: usize,
    pub min_possible_switches: usize,
    pub redundant_switches: usize,
    pub ping_pong_switches: usize,
    pub efficiency_pct: f64,
    pub opaque_after_blended: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortingAnalysis {
    pub total_switches: usize,
    pub min_possible_switches: usize,
    pub redundant_switches: usize,
    pub ping_pong_switches: usize,
    pub overall_efficiency_pct: f64,
    pub total_opaque_after_blended: usize,
    pub passes: Vec<PassSortingItem>,
    pub warnings: Vec<String>,
}

pub fn analyze_sorting(capture: &ParsedCapture) -> SortingAnalysis {
    let mut pass_items = Vec::new();

    // Group draws by pass
    // Draws in capture.draws correspond to the sequence in passes
    let mut draw_cursor = 0;

    for (pass_idx, pass) in capture.passes.iter().enumerate() {
        let count = pass.draw_count;
        if count == 0 {
            continue;
        }

        let end = (draw_cursor + count).min(capture.draws.len());
        let slice = &capture.draws[draw_cursor..end];
        draw_cursor = end;

        let progs: Vec<Option<u32>> = slice.iter().map(|d| d.program_id).collect();
        let mut switches: usize = 0;
        for i in 1..progs.len() {
            if progs[i] != progs[i - 1] {
                switches += 1;
            }
        }

        let unique_progs: FxHashSet<Option<u32>> = progs.iter().copied().collect();
        let min_switches = unique_progs.len().saturating_sub(1);
        let redundant = switches.saturating_sub(min_switches);

        let mut ping_pong = 0;
        for i in 0..progs.len().saturating_sub(2) {
            if progs[i] == progs[i + 2] && progs[i] != progs[i + 1] {
                ping_pong += 1;
            }
        }

        let efficiency = if count > 1 {
            ((count.saturating_sub(redundant)) as f64 / count as f64) * 100.0
        } else {
            100.0
        };

        let mut seen_blended = false;
        let mut opaque_after_blend = 0;
        for d in slice {
            if d.blend_enabled {
                seen_blended = true;
            } else if seen_blended && !d.blend_enabled {
                opaque_after_blend += 1;
            }
        }

        pass_items.push(PassSortingItem {
            pass_index: pass_idx,
            fbo_name: pass.fbo_name.clone(),
            draw_count: count,
            shader_switches: switches,
            min_possible_switches: min_switches,
            redundant_switches: redundant,
            ping_pong_switches: ping_pong,
            efficiency_pct: efficiency,
            opaque_after_blended: opaque_after_blend,
        });
    }

    let total_switches: usize = pass_items.iter().map(|p| p.shader_switches).sum();
    let min_possible: usize = pass_items.iter().map(|p| p.min_possible_switches).sum();
    let redundant_switches = total_switches.saturating_sub(min_possible);
    let ping_pong_switches: usize = pass_items.iter().map(|p| p.ping_pong_switches).sum();
    let total_opaque_after_blended: usize = pass_items.iter().map(|p| p.opaque_after_blended).sum();

    let total_draws: usize = pass_items.iter().map(|p| p.draw_count).sum();
    let overall_efficiency = if total_draws > 0 {
        ((total_draws.saturating_sub(redundant_switches)) as f64 / total_draws as f64) * 100.0
    } else {
        100.0
    };

    let mut warnings = Vec::new();

    if ping_pong_switches > 5 {
        warnings.push(format!(
            "{} ping-pong shader switches (A -> B -> A) detected. Draw calls are not sorted by material/shader, causing {} redundant shader switches per frame.",
            ping_pong_switches, redundant_switches
        ));
    }

    if total_opaque_after_blended > 0 {
        warnings.push(format!(
            "{} opaque draw calls were rendered AFTER transparent/blended draws in the same pass. This breaks Early-Z rejection, creates unnecessary pixel overdraw, and forces extra depth/blend state switches.",
            total_opaque_after_blended
        ));
    }

    SortingAnalysis {
        total_switches,
        min_possible_switches: min_possible,
        redundant_switches,
        ping_pong_switches,
        overall_efficiency_pct: overall_efficiency,
        total_opaque_after_blended,
        passes: pass_items,
        warnings,
    }
}
