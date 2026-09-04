pub mod draws;
pub mod glow;
pub mod passes;
pub mod particles;
pub mod shaders;
pub mod textures;
pub mod uploads;

pub use draws::*;
pub use glow::*;
pub use passes::*;
pub use particles::*;
pub use shaders::*;
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
    }
}
