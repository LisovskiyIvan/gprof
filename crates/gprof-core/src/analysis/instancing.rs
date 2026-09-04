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
    // Key: (vertex_buffer_bytes, triangles, program_id, fbo_name)
    // Meshes sharing the same geometry buffer size, triangle count, shader program and FBO
    type GroupKey = (usize, usize, Option<u32>, String);
    let mut groups: FxHashMap<GroupKey, Vec<usize>> = FxHashMap::default();

    for d in draws {
        if d.vertex_buffer_bytes > 0 && d.triangles > 0 {
            let key = (d.vertex_buffer_bytes, d.triangles, d.program_id, d.fbo_name.clone());
            groups.entry(key).or_default().push(d.index);
        }
    }

    let mut candidates = Vec::new();
    let mut total_saved = 0;

    for ((v_bytes, tris, pid, fbo), indices) in groups {
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
                fbo_name: fbo,
                draw_indices: indices,
                potential_saved_draws: saved,
            });
        }
    }

    candidates.sort_by(|a, b| b.draw_count.cmp(&a.draw_count).then_with(|| b.total_triangles.cmp(&a.total_triangles)));

    let rec = if total_saved > 0 {
        format!(
            "MERGE INTO INSTANCES: {} draw calls share identical geometry and shaders within their passes. Merging them into Babylon.js Thin Instances or Hardware Instances can save up to {} draw calls per frame.",
            candidates.iter().map(|c| c.draw_count).sum::<usize>(),
            total_saved
        )
    } else {
        "No duplicate geometry draw calls detected.".to_string()
    };

    InstancingAnalysisResult {
        total_candidates_groups: candidates.len(),
        total_potential_saved_draws: total_saved,
        candidates,
        recommendation: rec,
    }
}
