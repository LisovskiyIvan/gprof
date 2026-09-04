use crate::model::{DynamicBufferUploadItem, ParsedCapture};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index32BitMesh {
    pub draw_index: usize,
    pub command_id: usize,
    pub count: u32,
    pub triangles: usize,
    pub shader_name: String,
    pub fbo_name: String,
    pub world_position: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferAnalysis {
    pub total_buffer_memory_bytes: usize,
    pub total_dynamic_uploads: usize,
    pub duplicate_dynamic_uploads: usize,
    pub total_upload_bytes: usize,
    pub uploads: Vec<DynamicBufferUploadItem>,
    pub index_16bit_draws: usize,
    pub index_32bit_draws: usize,
    pub meshes_32bit: Vec<Index32BitMesh>,
    pub warnings: Vec<String>,
}

pub fn analyze(capture: &ParsedCapture) -> BufferAnalysis {
    let mut meshes_32bit = Vec::new();

    for d in &capture.draws {
        if d.index_type == Some(5125) {
            meshes_32bit.push(Index32BitMesh {
                draw_index: d.index,
                command_id: d.command_id,
                count: d.count,
                triangles: d.triangles,
                shader_name: d.shader_name.clone(),
                fbo_name: d.fbo_name.clone(),
                world_position: d.world_position,
            });
        }
    }

    let total_upload_bytes: usize = capture.buffer_uploads.iter().map(|u| u.size_bytes).sum();
    let duplicate_dynamic_uploads = capture.buffer_uploads.iter().filter(|u| u.is_duplicate).count();

    let mut warnings = Vec::new();

    if capture.summary.buffer_subdata_calls > 0 {
        if duplicate_dynamic_uploads > 0 {
            warnings.push(format!(
                "{} duplicate bufferSubData uploads per frame. Identical data is repeatedly sent to the GPU.",
                duplicate_dynamic_uploads
            ));
        }
        warnings.push(format!(
            "{} dynamic buffer uploads per frame ({} bytes). Check stack traces to batch dynamic buffer updates.",
            capture.summary.buffer_subdata_calls, total_upload_bytes
        ));
    }

    if !meshes_32bit.is_empty() {
        warnings.push(format!(
            "{} draw calls use 32-bit index buffers (UNSIGNED_INT >65k vertices). This triggers VERTEX_PULLING_INDEX_BUFFER_32BITS shader variants and consumes 2x IBO memory bandwidth. Consider splitting into submeshes.",
            meshes_32bit.len()
        ));
    }

    BufferAnalysis {
        total_buffer_memory_bytes: capture.memory.buffer_memory_bytes,
        total_dynamic_uploads: capture.buffer_uploads.len(),
        duplicate_dynamic_uploads,
        total_upload_bytes,
        uploads: capture.buffer_uploads.clone(),
        index_16bit_draws: capture.summary.index_16bit_draws,
        index_32bit_draws: capture.summary.index_32bit_draws,
        meshes_32bit,
        warnings,
    }
}
