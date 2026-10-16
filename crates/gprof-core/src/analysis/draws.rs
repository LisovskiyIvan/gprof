use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeavyDrawCall {
    pub index: usize,
    pub command_id: usize,
    pub triangles: usize,
    pub points: usize,
    pub fbo_name: String,
    pub shader_name: String,
    pub world_position: Option<[f32; 3]>,
    pub vertex_buffer_bytes: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawsAnalysisResult {
    pub total_draws: usize,
    pub total_triangles: usize,
    pub total_points: usize,
    pub heaviest_draws: Vec<HeavyDrawCall>,
}

pub fn analyze_draws(draws: &[DrawCallItem], top_n: usize) -> DrawsAnalysisResult {
    let total_draws = draws.len();
    let total_triangles = draws.iter().map(|d| d.triangles).sum();
    let total_points = draws.iter().map(|d| d.points).sum();

    let mut sorted: Vec<&DrawCallItem> = draws.iter().collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.triangles));

    let heaviest: Vec<HeavyDrawCall> = sorted
        .into_iter()
        .take(top_n)
        .map(|d| HeavyDrawCall {
            index: d.index,
            command_id: d.command_id,
            triangles: d.triangles,
            points: d.points,
            fbo_name: d.fbo_name.clone(),
            shader_name: d.shader_name.clone(),
            world_position: d.world_position,
            vertex_buffer_bytes: d.vertex_buffer_bytes,
            text: d.text.clone(),
        })
        .collect();

    DrawsAnalysisResult {
        total_draws,
        total_triangles,
        total_points,
        heaviest_draws: heaviest,
    }
}
