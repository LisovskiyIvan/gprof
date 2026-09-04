pub mod buffers;
pub mod compare;
pub mod draws;
pub mod glow;
pub mod instancing;
pub mod passes;
pub mod particles;
pub mod shaders;
pub mod states;
pub mod textures;
pub mod uploads;

pub use buffers::*;
pub use compare::*;
pub use draws::*;
pub use glow::*;
pub use instancing::*;
pub use passes::*;
pub use particles::*;
pub use shaders::*;
pub use states::*;
pub use textures::*;
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
}

pub fn analyze(capture: &ParsedCapture, top_draws: usize) -> ComprehensiveAnalysis {
    ComprehensiveAnalysis {
        textures: textures::analyze_textures(&capture.textures),
        glow: glow::audit_glow_layer(&capture.draws),
        draws: draws::analyze_draws(&capture.draws, top_draws),
        passes: passes::analyze_passes(&capture.passes),
        shaders: shaders::analyze_shaders(
            &capture.programs,
            capture.summary.use_program_calls,
            capture.summary.redundant_use_program,
        ),
        particles: particles::analyze_particles(&capture.particles),
        uploads: uploads::analyze_uploads(&capture.uploads),
        instancing: instancing::analyze_instancing(&capture.draws),
        states: states::analyze(capture),
        buffers: buffers::analyze(capture),
    }
}
