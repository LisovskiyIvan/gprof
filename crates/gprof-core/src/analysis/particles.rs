use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticlesAnalysisResult {
    pub systems_count: usize,
    pub total_particles: usize,
    pub total_quad_triangles: usize,
    pub systems: Vec<ParticleSystemItem>,
    pub warning: Option<String>,
}

pub fn analyze_particles(particles: &[ParticleSystemItem]) -> ParticlesAnalysisResult {
    let systems_count = particles.len();
    let total_particles: usize = particles.iter().map(|p| p.particle_count).sum();
    let total_quad_triangles: usize = particles.iter().map(|p| p.quads_rendered).sum();

    let warning = if total_particles > 50_000 {
        Some(format!(
            "HEAVY PARTICLE LOAD: {} active GPU particles across {} systems ({} billboard triangles/frame). Large additive particle counts cause severe fill-rate drops and overdraw.",
            total_particles, systems_count, total_quad_triangles
        ))
    } else {
        None
    };

    ParticlesAnalysisResult {
        systems_count,
        total_particles,
        total_quad_triangles,
        systems: particles.to_vec(),
        warning,
    }
}
