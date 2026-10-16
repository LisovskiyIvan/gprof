use crate::model::ParsedCapture;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaqueAfterBlendCandidate {
    pub pass_index: usize,
    pub draw_index: usize,
    pub shader_name: String,
    pub triangles: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassSortingItem {
    pub pass_index: usize,
    pub fbo_name: String,
    pub draw_count: usize,
    pub shader_switches: usize,
    pub min_possible_switches: usize,
    pub reducible_switches: usize,
    pub redundant_switches: usize, // backwards-compatible alias
    pub ping_pong_switches: usize,
    pub efficiency_pct: f64,
    pub opaque_after_blended: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortingAnalysis {
    pub total_switches: usize,
    pub min_possible_switches: usize,
    pub reducible_switches: usize,
    pub redundant_switches: usize, // backwards-compatible alias
    pub api_redundant_use_program: usize,
    pub ping_pong_switches: usize,
    pub overall_efficiency_pct: f64,
    pub total_opaque_after_blended: usize,
    pub opaque_after_blended_draws: Vec<OpaqueAfterBlendCandidate>,
    pub passes: Vec<PassSortingItem>,
    pub warnings: Vec<String>,
}

pub fn analyze_sorting(capture: &ParsedCapture) -> SortingAnalysis {
    let mut pass_items = Vec::new();
    let mut opaque_candidates = Vec::new();

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

        // Phase-aware min switches calculation:
        // Draws inside a pass cannot be freely reordered across phase boundaries:
        // Phase 1: Opaque before blend
        // Phase 2: Alpha-blended
        // Phase 3: Opaque after blend (potential overlays or HUD)
        let mut p1_progs = Vec::new();
        let mut p2_progs = Vec::new();
        let mut p3_progs = Vec::new();
        let mut seen_blended = false;

        for d in slice {
            if d.blend_enabled {
                seen_blended = true;
                p2_progs.push(d.program_id);
            } else if !seen_blended {
                p1_progs.push(d.program_id);
            } else {
                p3_progs.push(d.program_id);
                opaque_candidates.push(OpaqueAfterBlendCandidate {
                    pass_index: pass_idx,
                    draw_index: d.index,
                    shader_name: d.shader_name.clone(),
                    triangles: d.triangles,
                });
            }
        }

        let mut phase_constrained_min = 0;
        let mut active_phases = 0;

        if !p1_progs.is_empty() {
            active_phases += 1;
            let unique: FxHashSet<Option<u32>> = p1_progs.into_iter().collect();
            phase_constrained_min += unique.len().saturating_sub(1);
        }
        if !p2_progs.is_empty() {
            active_phases += 1;
            let unique: FxHashSet<Option<u32>> = p2_progs.into_iter().collect();
            phase_constrained_min += unique.len().saturating_sub(1);
        }
        if !p3_progs.is_empty() {
            active_phases += 1;
            let unique: FxHashSet<Option<u32>> = p3_progs.into_iter().collect();
            phase_constrained_min += unique.len().saturating_sub(1);
        }
        if active_phases > 1 {
            phase_constrained_min += active_phases - 1;
        }

        let min_switches = phase_constrained_min.min(switches);
        let reducible = switches.saturating_sub(min_switches);

        let mut ping_pong = 0;
        for i in 0..progs.len().saturating_sub(2) {
            if progs[i] == progs[i + 2] && progs[i] != progs[i + 1] {
                ping_pong += 1;
            }
        }

        let efficiency = if count > 1 { ((count.saturating_sub(reducible)) as f64 / count as f64) * 100.0 } else { 100.0 };

        let opaque_after_blend = slice.iter().filter(|d| seen_blended && !d.blend_enabled).count();

        pass_items.push(PassSortingItem {
            pass_index: pass_idx,
            fbo_name: pass.fbo_name.clone(),
            draw_count: count,
            shader_switches: switches,
            min_possible_switches: min_switches,
            reducible_switches: reducible,
            redundant_switches: reducible,
            ping_pong_switches: ping_pong,
            efficiency_pct: efficiency,
            opaque_after_blended: opaque_after_blend,
        });
    }

    let total_switches: usize = pass_items.iter().map(|p| p.shader_switches).sum();
    let min_possible: usize = pass_items.iter().map(|p| p.min_possible_switches).sum();
    let reducible_switches = total_switches.saturating_sub(min_possible);
    let ping_pong_switches: usize = pass_items.iter().map(|p| p.ping_pong_switches).sum();
    let total_opaque_after_blended: usize = pass_items.iter().map(|p| p.opaque_after_blended).sum();
    let api_redundant_use_program = capture.summary.redundant_use_program;

    let total_draws: usize = pass_items.iter().map(|p| p.draw_count).sum();
    let overall_efficiency = if total_draws > 0 {
        ((total_draws.saturating_sub(reducible_switches)) as f64 / total_draws as f64) * 100.0
    } else {
        100.0
    };

    let mut warnings = Vec::new();

    if ping_pong_switches > 5 {
        warnings.push(format!(
            "[Calculated Estimate] {} ping-pong shader switches (A -> B -> A) between draw calls. Reordering draws by material/shader within each phase can eliminate up to {} reducible program switches.",
            ping_pong_switches, reducible_switches
        ));
    }

    if total_opaque_after_blended > 0 {
        let sample_draws = opaque_candidates.iter().take(4).map(|c| format!("#{}", c.draw_index)).collect::<Vec<_>>().join(", ");
        warnings.push(format!(
            "[Hypothesis / Candidate] {} opaque draw calls (e.g. {}) were submitted after alpha-blended draws in the same pass. If these are 2D UI elements, overlays, or depth-untested decals, this order is intentional. If they represent 3D scene geometry, moving them prior to the alpha-blend phase will enable hardware Early-Z depth rejection.",
            total_opaque_after_blended, sample_draws
        ));
    }

    SortingAnalysis {
        total_switches,
        min_possible_switches: min_possible,
        reducible_switches,
        redundant_switches: reducible_switches,
        api_redundant_use_program,
        ping_pong_switches,
        overall_efficiency_pct: overall_efficiency,
        total_opaque_after_blended,
        opaque_after_blended_draws: opaque_candidates,
        passes: pass_items,
        warnings,
    }
}
