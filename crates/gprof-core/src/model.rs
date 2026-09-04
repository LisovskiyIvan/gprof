use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanvasInfo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub client_width: Option<u32>,
    pub client_height: Option<u32>,
    pub browser_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextInfo {
    pub version: Option<u32>,
    pub renderer: Option<String>,
    pub vendor: Option<String>,
    pub shading_language_version: Option<String>,
    pub max_texture_size: Option<u32>,
    pub max_draw_buffers: Option<u32>,
    pub compressed_formats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureItem {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub format_name: String,
    pub internal_format: u32,
    pub internal_format_name: String,
    pub r#type: u32,
    pub type_name: String,
    pub length: usize,
    pub is_compressed: bool,
    pub target: String,
    pub draw_count: usize,
    pub bound_units: Vec<u32>,
    pub uniform_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCallItem {
    pub index: usize,
    pub command_id: usize,
    pub name: String,
    pub text: String,
    pub mode: u32,
    pub mode_name: String,
    pub count: u32,
    pub instance_count: u32,
    pub triangles: usize,
    pub points: usize,
    pub lines: usize,
    pub fbo_name: String,
    pub program_id: Option<u32>,
    pub shader_name: String,
    pub is_glow_pass: bool,
    pub is_black_glow: bool,
    pub glow_color: Option<[f32; 4]>,
    pub world_position: Option<[f32; 3]>,
    pub vertex_buffer_bytes: usize,
    pub vertex_count_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPassItem {
    pub pass_index: usize,
    pub fbo_name: String,
    pub pass_type: String,
    pub draw_count: usize,
    pub triangles: usize,
    pub points: usize,
    pub shaders: Vec<String>,
    pub start_command: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramItem {
    pub id: u32,
    pub draw_count: usize,
    pub shader_name: String,
    pub defines: Vec<String>,
    pub is_pbr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSystemItem {
    pub update_draw_index: usize,
    pub render_draw_index: Option<usize>,
    pub particle_count: usize,
    pub quads_rendered: usize,
    pub is_transform_feedback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicUploadItem {
    pub command_index: usize,
    pub name: String,
    pub target: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub r#type: u32,
    pub stack_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameCaptureSummary {
    pub total_commands: usize,
    pub draw_calls: usize,
    pub draw_elements: usize,
    pub draw_elements_instanced: usize,
    pub draw_arrays: usize,
    pub draw_arrays_instanced: usize,
    pub triangles: usize,
    pub points: usize,
    pub lines: usize,
    pub unique_programs: usize,
    pub use_program_calls: usize,
    pub redundant_use_program: usize,
    pub texture_bind_calls: usize,
    pub fbo_bind_calls: usize,
    pub active_fbo_count: usize,
    pub dynamic_uploads_count: usize,
    pub total_texture_bytes: usize,
    pub total_textures_count: usize,
    pub glow_total_draws: usize,
    pub glow_black_draws: usize,
    pub glow_active_draws: usize,
    pub glow_total_triangles: usize,
    pub glow_black_triangles: usize,
    pub glow_active_triangles: usize,
    pub particle_systems_count: usize,
    pub total_particles: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCapture {
    pub canvas: Option<CanvasInfo>,
    pub context: Option<ContextInfo>,
    pub summary: FrameCaptureSummary,
    pub textures: Vec<TextureItem>,
    pub draws: Vec<DrawCallItem>,
    pub passes: Vec<RenderPassItem>,
    pub programs: Vec<ProgramItem>,
    pub particles: Vec<ParticleSystemItem>,
    pub uploads: Vec<DynamicUploadItem>,
}
