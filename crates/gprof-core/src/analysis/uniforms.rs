use crate::model::ParsedCapture;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformStats {
    pub total_calls: usize,
    pub redundant_calls: usize,
    pub redundancy_pct: f64,
    pub total_bytes: usize,
    pub redundant_bytes: usize,
    pub matrix_calls: usize,
    pub redundant_matrix_calls: usize,
    pub avg_uniforms_per_draw: f64,
    pub avg_api_calls_per_draw: f64,
    pub avg_active_uniforms_per_draw: f64,
    pub max_uniforms_in_draw: usize,
    pub top_redundant_callers: Vec<UniformCallerStat>,
    pub redundant_by_type: Vec<UniformTypeStat>,
    pub heaviest_uniform_draws: Vec<DrawUniformStat>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformCallerStat {
    pub caller: String,
    pub total_calls: usize,
    pub redundant_calls: usize,
    pub redundant_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformTypeStat {
    pub type_name: String,
    pub total: usize,
    pub redundant: usize,
    pub redundant_pct: f64,
    pub bytes: usize,
    pub redundant_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawUniformStat {
    pub draw_index: usize,
    pub uniform_count: usize,
    pub shader_name: String,
    pub fbo_name: String,
    pub triangles: usize,
}

pub fn analyze_uniforms(capture: &ParsedCapture) -> UniformStats {
    let total_calls = capture.summary.uniform_calls;
    let redundant_calls = capture.summary.redundant_uniform_calls;
    let redundancy_pct = if total_calls > 0 {
        (redundant_calls as f64 / total_calls as f64) * 100.0
    } else {
        0.0
    };

    let total_bytes = capture.summary.uniform_bytes;
    let redundant_bytes = capture.summary.redundant_uniform_bytes;

    let mut matrix_calls = 0;
    let mut redundant_matrix_calls = 0;

    let mut by_type_map: FxHashMap<String, (usize, usize, usize, usize)> = FxHashMap::default();
    let mut caller_map: FxHashMap<String, (usize, usize)> = FxHashMap::default();

    for u in &capture.uniform_commands {
        let is_matrix = u.name.contains("Matrix");
        if is_matrix {
            matrix_calls += 1;
            if u.is_redundant {
                redundant_matrix_calls += 1;
            }
        }

        let entry = by_type_map.entry(u.name.clone()).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        entry.2 += u.bytes;
        if u.is_redundant {
            entry.1 += 1;
            entry.3 += u.bytes;
        }

        let caller = u
            .stack_caller
            .clone()
            .unwrap_or_else(|| "Unknown Call Site".to_string());
        let c_entry = caller_map.entry(caller).or_insert((0, 0));
        c_entry.0 += 1;
        if u.is_redundant {
            c_entry.1 += 1;
        }
    }

    let mut redundant_by_type: Vec<UniformTypeStat> = by_type_map
        .into_iter()
        .map(|(type_name, (total, redundant, bytes, redundant_bytes))| {
            let red_pct = if total > 0 {
                (redundant as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            UniformTypeStat {
                type_name,
                total,
                redundant,
                redundant_pct: red_pct,
                bytes,
                redundant_bytes,
            }
        })
        .collect();
    redundant_by_type.sort_by(|a, b| b.redundant.cmp(&a.redundant).then_with(|| b.total.cmp(&a.total)));

    let mut top_redundant_callers: Vec<UniformCallerStat> = caller_map
        .into_iter()
        .map(|(caller, (total_calls, redundant_calls))| {
            let red_pct = if total_calls > 0 {
                (redundant_calls as f64 / total_calls as f64) * 100.0
            } else {
                0.0
            };
            UniformCallerStat {
                caller,
                total_calls,
                redundant_calls,
                redundant_pct: red_pct,
            }
        })
        .filter(|c| c.redundant_calls > 0)
        .collect();
    top_redundant_callers.sort_by(|a, b| b.redundant_calls.cmp(&a.redundant_calls));

    // Draw uniforms stats: count declared active uniform variables on the shader for that draw
    let mut total_draw_uniforms = 0;
    let mut max_uniforms_in_draw = 0;
    let mut heaviest_uniform_draws = Vec::new();

    for draw in &capture.draws {
        total_draw_uniforms += draw.uniform_count;
        if draw.uniform_count > max_uniforms_in_draw {
            max_uniforms_in_draw = draw.uniform_count;
        }
        heaviest_uniform_draws.push(DrawUniformStat {
            draw_index: draw.index,
            uniform_count: draw.uniform_count,
            shader_name: draw.shader_name.clone(),
            fbo_name: draw.fbo_name.clone(),
            triangles: draw.triangles,
        });
    }

    let avg_active_uniforms_per_draw = if !capture.draws.is_empty() {
        total_draw_uniforms as f64 / capture.draws.len() as f64
    } else {
        0.0
    };

    let avg_api_calls_per_draw = if !capture.draws.is_empty() {
        total_calls as f64 / capture.draws.len() as f64
    } else {
        0.0
    };

    let avg_uniforms_per_draw = avg_active_uniforms_per_draw;

    heaviest_uniform_draws.sort_by(|a, b| b.uniform_count.cmp(&a.uniform_count));
    heaviest_uniform_draws.truncate(15);

    // Generate actionable recommendations
    let mut recommendations = Vec::new();
    if redundancy_pct >= 20.0 {
        recommendations.push(format!(
            "[Calculated Estimate] Redundant uniform dispatch: {:.1}% of gl.uniform* API calls ({} of {}) transmitted identical data to the driver ({} CPU-to-driver payload). Eliminating redundant uniform dispatches reduces CPU driver validation overhead.",
            redundancy_pct, redundant_calls, total_calls, crate::report::format_bytes(redundant_bytes)
        ));
    }

    if redundant_matrix_calls > 0 {
        recommendations.push(format!(
            "[Hypothesis / Candidate] Matrix payload: {} redundant uniformMatrix4fv calls detected ({} CPU-to-driver payload). If world/viewProjection matrices do not change between draws, add dirty checking on the CPU or migrate to Uniform Buffer Objects (UBOs) in WebGL 2.",
            redundant_matrix_calls, crate::report::format_bytes(redundant_bytes)
        ));
    }

    if let Some(top) = top_redundant_callers.first() {
        if top.redundant_calls > 5 {
            recommendations.push(format!(
                "[Observed Fact] Caller hotspot: '{}' issued {} redundant uniform uploads ({:.1}% redundant).",
                top.caller, top.redundant_calls, top.redundant_pct
            ));
        }
    }

    recommendations.push(format!(
        "[Observed Fact] Uniform frequency: average {:.2} gl.uniform* API calls per draw call (total {} calls across {} draws), while shaders declare an average of {:.1} active uniform variables (max {} in a single draw).",
        avg_api_calls_per_draw, total_calls, capture.draws.len(), avg_active_uniforms_per_draw, max_uniforms_in_draw
    ));

    UniformStats {
        total_calls,
        redundant_calls,
        redundancy_pct,
        total_bytes,
        redundant_bytes,
        matrix_calls,
        redundant_matrix_calls,
        avg_uniforms_per_draw,
        avg_api_calls_per_draw,
        avg_active_uniforms_per_draw,
        max_uniforms_in_draw,
        top_redundant_callers,
        redundant_by_type,
        heaviest_uniform_draws,
        recommendations,
    }
}

