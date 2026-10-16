pub mod buffers;
pub mod compare;
pub mod compression;
pub mod draws;
pub mod fbo;
pub mod glow;
pub mod instancing;
pub mod lint;
pub mod overdraw;
pub mod particles;
pub mod passes;
pub mod scene_tree;
pub mod shaders;
pub mod sorting;
pub mod states;
pub mod textures;
pub mod uniforms;
pub mod uploads;

pub use buffers::*;
pub use compare::*;
pub use compression::*;
pub use draws::*;
pub use fbo::*;
pub use glow::*;
pub use instancing::*;
pub use lint::*;
pub use overdraw::*;
pub use particles::*;
pub use passes::*;
pub use scene_tree::*;
pub use shaders::*;
pub use sorting::*;
pub use states::*;
pub use textures::*;
pub use uniforms::*;
pub use uploads::*;

use crate::model::ParsedCapture;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveAnalysis {
    pub textures: TextureAnalysisResult,
    pub glow: GlowAuditResult,
    pub draws: DrawsAnalysisResult,
    pub passes: PassesAnalysisResult,
    pub shaders: ShadersAnalysisResult,
    pub particles: ParticlesAnalysisResult,
    pub uploads: UploadsAnalysisResult,
    pub instancing: InstancingAnalysisResult,
    pub states: StateAnalysis,
    pub buffers: BufferAnalysis,
    pub compression: CompressionAnalysis,
    pub fbos: FboAnalysis,
    pub sorting: SortingAnalysis,
    pub uniforms: UniformStats,
    pub overdraw: OverdrawAnalysisResult,
    pub tree: SceneTreeAnalysisResult,
    pub lint: LintReport,
}

pub fn analyze(capture: &ParsedCapture, top_draws: usize) -> ComprehensiveAnalysis {
    let mut partial = ComprehensiveAnalysis {
        textures: textures::analyze_textures(&capture.textures),
        glow: glow::audit_glow_layer(&capture.draws),
        draws: draws::analyze_draws(&capture.draws, top_draws),
        passes: passes::analyze_passes(&capture.passes),
        shaders: shaders::analyze_shaders(&capture.programs, capture.summary.use_program_calls, capture.summary.redundant_use_program),
        particles: particles::analyze_particles(&capture.particles),
        uploads: uploads::analyze_uploads(&capture.uploads),
        instancing: instancing::analyze_instancing(&capture.draws),
        states: states::analyze(capture),
        buffers: buffers::analyze(capture),
        compression: compression::analyze_compression(&capture.textures),
        fbos: fbo::analyze_fbos(&capture.passes),
        sorting: sorting::analyze_sorting(capture),
        uniforms: uniforms::analyze_uniforms(capture),
        overdraw: overdraw::analyze_overdraw(capture),
        tree: scene_tree::analyze_scene_tree(capture),
        lint: LintReport::default(),
    };

    partial.lint = lint::run_linter(capture, &partial);
    partial
}
