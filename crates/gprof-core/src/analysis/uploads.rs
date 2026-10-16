use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadsAnalysisResult {
    pub total_uploads: usize,
    pub uploads: Vec<DynamicUploadItem>,
    pub warning: Option<String>,
}

pub fn analyze_uploads(uploads: &[DynamicUploadItem]) -> UploadsAnalysisResult {
    let total_uploads = uploads.len();
    let tex_image_calls = uploads.iter().filter(|u| u.name == "texImage2D").count();

    let warning = if tex_image_calls > 0 {
        let first_st = uploads.iter().find(|u| !u.stack_trace.is_empty()).and_then(|u| u.stack_trace.first()).cloned().unwrap_or_default();
        Some(format!(
            "PER-FRAME TEXTURE ALLOCATION: {} `texImage2D` call(s) executed per frame. `texImage2D` reallocates texture storage on the GPU. Switch to `texSubImage2D` or uniform arrays. Top stack: {}",
            tex_image_calls, first_st
        ))
    } else {
        None
    };

    UploadsAnalysisResult {
        total_uploads,
        uploads: uploads.to_vec(),
        warning,
    }
}
