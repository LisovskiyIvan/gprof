use crate::model::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancingCandidate {
    pub position_buffer_bytes: usize,
    pub triangles_per_instance: usize,
    pub draw_count: usize,
    pub total_triangles: usize,
    pub program_id: Option<u32>,
    pub pass_index: usize,
    pub fbo_name: String,
    pub draw_indices: Vec<usize>,
    pub potential_saved_draws: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancingAnalysisResult {
    pub total_candidates_groups: usize,
    pub total_potential_saved_draws: usize,
    pub candidates: Vec<InstancingCandidate>,
    pub recommendation: String,
}

pub fn analyze_instancing(draws: &[DrawCallItem]) -> InstancingAnalysisResult {
    // Key: (vertex_buffer_bytes, triangles, program_id, pass_index, fbo_name)
    // Meshes sharing the same geometry buffer size, triangle count, shader program,
    // and occurring within the SAME render pass. We do not group draws across different
    // passes (e.g. Glow pass vs main scene).
    type GroupKey = (usize, usize, Option<u32>, usize, String);
    let mut groups: FxHashMap<GroupKey, Vec<usize>> = FxHashMap::default();

    for d in draws {
        if d.vertex_buffer_bytes > 0 && d.triangles > 0 {
            let key = (d.vertex_buffer_bytes, d.triangles, d.program_id, d.pass_index, d.fbo_name.clone());
            groups.entry(key).or_default().push(d.index);
        }
    }

    let mut candidates = Vec::new();
    let mut total_saved = 0;

    for ((v_bytes, tris, pid, pass_idx, fbo), indices) in groups {
        if indices.len() > 1 {
            let count = indices.len();
            let saved = count - 1;
            total_saved += saved;
            candidates.push(InstancingCandidate {
                position_buffer_bytes: v_bytes,
                triangles_per_instance: tris,
                draw_count: count,
                total_triangles: tris * count,
                program_id: pid,
                pass_index: pass_idx,
                fbo_name: fbo,
                draw_indices: indices,
                potential_saved_draws: saved,
            });
        }
    }

    candidates.sort_by(|a, b| b.draw_count.cmp(&a.draw_count).then_with(|| b.total_triangles.cmp(&a.total_triangles)));

    let rec = if total_saved > 0 {
        format!(
            "[Hypothesis / Candidate] Instancing upper-bound estimate: {} draw calls across {} groups share identical geometry buffers and shaders within their render passes. Merging identical meshes within each pass into Thin Instances or Hardware Instances could theoretically save up to {} draw calls per frame (actual feasibility depends on dynamic material uniforms and culling requirements).",
            candidates.iter().map(|c| c.draw_count).sum::<usize>(),
            candidates.len(),
            total_saved
        )
    } else {
        "[Observed Fact] No duplicate geometry draw calls detected within individual render passes.".to_string()
    };

    InstancingAnalysisResult {
        total_candidates_groups: candidates.len(),
        total_potential_saved_draws: total_saved,
        candidates,
        recommendation: rec,
    }
}

