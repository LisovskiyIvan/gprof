pub mod buffers;
pub mod compare;
pub mod compression;
pub mod draws;
pub mod fbo;
pub mod glow;
pub mod instancing;
pub mod lint;
pub mod overdraw;
pub mod passes;
pub mod particles;
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
pub use passes::*;
pub use particles::*;
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
    let textures = textures::analyze_textures(&capture.textures);
    let glow = glow::audit_glow_layer(&capture.draws);
    let draws = draws::analyze_draws(&capture.draws, top_draws);
    let passes = passes::analyze_passes(&capture.passes);
    let shaders = shaders::analyze_shaders(
        &capture.programs,
        capture.summary.use_program_calls,
        capture.summary.redundant_use_program,
    );
    let particles = particles::analyze_particles(&capture.particles);
    let uploads = uploads::analyze_uploads(&capture.uploads);
    let instancing = instancing::analyze_instancing(&capture.draws);
    let states = states::analyze(capture);
    let buffers = buffers::analyze(capture);
    let compression = compression::analyze_compression(&capture.textures);
    let fbos = fbo::analyze_fbos(&capture.passes);
    let sorting = sorting::analyze_sorting(capture);
    let uniforms = uniforms::analyze_uniforms(capture);
    let overdraw = overdraw::analyze_overdraw(capture);
    let tree = scene_tree::analyze_scene_tree(capture);

    let mut partial = ComprehensiveAnalysis {
        textures,
        glow,
        draws,
        passes,
        shaders,
        particles,
        uploads,
        instancing,
        states,
        buffers,
        compression,
        fbos,
        sorting,
        uniforms,
        overdraw,
        tree,
        lint: LintReport {
            total_issues: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            rules: Vec::new(),
        },
    };

    partial.lint = lint::run_linter(capture, &partial);
    partial
}

