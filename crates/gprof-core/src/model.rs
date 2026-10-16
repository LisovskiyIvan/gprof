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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionStatus {
    Compressed,
    Uncompressed,
    Unknown,
}

impl CompressionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionStatus::Compressed => "Yes",
            CompressionStatus::Uncompressed => "No",
            CompressionStatus::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureEstimationMethod {
    StorageSize,       // Exact known length from storage/upload
    FormatCalculation, // Calculated from known internalFormat / format / type
    EstimatedFallback, // Assumed fallback (e.g. 4 B/px) due to missing format
    Unknown,           // Insufficient data
}

impl TextureEstimationMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextureEstimationMethod::StorageSize => "Storage (exact)",
            TextureEstimationMethod::FormatCalculation => "Calculated (format)",
            TextureEstimationMethod::EstimatedFallback => "Estimated (fallback)",
            TextureEstimationMethod::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureUsageCategory {
    ColorMaterial,    // Albedo, diffuse, normal, roughness, emissive, metallic, pbr
    GpuAnimationData, // VAT (Vertex Animation Texture), bone palettes, matrix textures, float data
    RenderTarget,     // FBO attachments, depth, bloom/blur
    Unknown,          // Unidentified purpose
}

impl TextureUsageCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextureUsageCategory::ColorMaterial => "Color / Material",
            TextureUsageCategory::GpuAnimationData => "GPU Animation / Data",
            TextureUsageCategory::RenderTarget => "Render Target / PostProcess",
            TextureUsageCategory::Unknown => "Unknown / Unclassified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScissorState {
    pub enabled: bool,
    pub rect: Rect,
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
    pub base_bytes: usize,
    pub mip_levels: usize,
    pub is_compressed: bool,
    pub compression_status: CompressionStatus,
    pub estimation_method: TextureEstimationMethod,
    pub category: TextureUsageCategory,
    pub target: String,
    pub draw_count: usize,
    pub bound_units: Vec<u32>,
    pub uniform_names: Vec<String>,
    pub is_pot: bool,
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
    pub pass_index: usize,
    pub program_id: Option<u32>,
    pub shader_name: String,
    pub is_glow_pass: bool,
    pub is_black_glow: bool,
    pub glow_color: Option<[f32; 4]>,
    pub world_position: Option<[f32; 3]>,
    pub vertex_buffer_bytes: usize,
    pub vertex_count_estimate: usize,
    pub is_instanced: bool,
    pub index_type: Option<u32>,
    pub index_type_name: Option<String>,
    pub blend_enabled: bool,
    pub depth_write: bool,
    pub cull_enabled: bool,
    pub uniform_count: usize,
    pub active_uniform_count: usize,
    pub samplers: Vec<String>,
    pub stack_component: Option<String>,
    pub viewport: Rect,
    pub scissor: ScissorState,
    pub drawing_buffer: (u32, u32),
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
    pub viewport_width: Option<u32>,
    pub viewport_height: Option<u32>,
    pub drawing_buffer: (u32, u32),
    pub scissor: Option<ScissorState>,
    pub vram_bytes: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicBufferUploadItem {
    pub command_index: usize,
    pub name: String,
    pub target: u32,
    pub target_name: String,
    pub offset: usize,
    pub size_bytes: usize,
    pub stack_trace: Vec<String>,
    pub data_sample: Option<String>,
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformCommandItem {
    pub command_index: usize,
    pub name: String,
    pub location_id: Option<u32>,
    pub bytes: usize,
    pub is_redundant: bool,
    pub stack_caller: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateChangeSummary {
    pub depth_mask_calls: usize,
    pub redundant_depth_mask: usize,
    pub color_mask_calls: usize,
    pub redundant_color_mask: usize,
    pub blend_toggle_calls: usize,
    pub redundant_blend_toggles: usize,
    pub cull_toggle_calls: usize,
    pub redundant_cull_toggles: usize,
    pub depth_test_calls: usize,
    pub redundant_depth_test: usize,
    pub fbo_bind_calls: usize,
    pub redundant_fbo_binds: usize,
    pub texture_bind_calls: usize,
    pub redundant_texture_binds: usize,
    pub vao_bind_calls: usize,
    pub redundant_vao_binds: usize,
    pub use_program_calls: usize,
    pub redundant_use_program: usize,
    pub total_redundant_calls: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemorySummary {
    pub buffer_memory_bytes: usize,
    pub texture_memory_bytes: usize,
    pub program_memory_bytes: usize,
    pub renderbuffer_memory_bytes: usize,
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
    pub blended_draws: usize,
    pub opaque_draws: usize,
    pub index_32bit_draws: usize,
    pub index_16bit_draws: usize,
    pub unique_programs: usize,
    pub use_program_calls: usize,
    pub redundant_use_program: usize,
    pub texture_bind_calls: usize,
    pub fbo_bind_calls: usize,
    pub active_fbo_count: usize,
    pub dynamic_uploads_count: usize,
    pub buffer_subdata_calls: usize,
    pub duplicate_buffer_uploads: usize,
    pub uniform_calls: usize,
    pub redundant_uniform_calls: usize,
    pub uniform_bytes: usize,
    pub redundant_uniform_bytes: usize,
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
    pub state_changes: StateChangeSummary,
    pub memory: MemorySummary,
    pub textures: Vec<TextureItem>,
    pub draws: Vec<DrawCallItem>,
    pub passes: Vec<RenderPassItem>,
    pub programs: Vec<ProgramItem>,
    pub particles: Vec<ParticleSystemItem>,
    pub uploads: Vec<DynamicUploadItem>,
    pub buffer_uploads: Vec<DynamicBufferUploadItem>,
    pub uniform_commands: Vec<UniformCommandItem>,
}
