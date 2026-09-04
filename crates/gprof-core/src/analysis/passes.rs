use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassesAnalysisResult {
    pub total_passes: usize,
    pub passes: Vec<RenderPassItem>,
}

pub fn analyze_passes(passes: &[RenderPassItem]) -> PassesAnalysisResult {
    PassesAnalysisResult {
        total_passes: passes.len(),
        passes: passes.to_vec(),
    }
}
