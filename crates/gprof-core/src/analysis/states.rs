use crate::model::ParsedCapture;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateItem {
    pub name: String,
    pub total_calls: usize,
    pub redundant_calls: usize,
    pub redundant_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAnalysis {
    pub total_state_calls: usize,
    pub total_redundant_calls: usize,
    pub overall_redundancy_pct: f64,
    pub items: Vec<StateItem>,
    pub opaque_draws: usize,
    pub blended_draws: usize,
    pub warnings: Vec<String>,
}

pub fn analyze(capture: &ParsedCapture) -> StateAnalysis {
    let sc = &capture.state_changes;
    let mut items = Vec::new();

    let mut push_item = |name: &str, total: usize, redundant: usize| {
        let pct = if total > 0 { (redundant as f64 / total as f64) * 100.0 } else { 0.0 };
        items.push(StateItem {
            name: name.to_string(),
            total_calls: total,
            redundant_calls: redundant,
            redundant_pct: pct,
        });
    };

    push_item("depthMask", sc.depth_mask_calls, sc.redundant_depth_mask);
    push_item("enable/disable (BLEND)", sc.blend_toggle_calls, sc.redundant_blend_toggles);
    push_item("enable/disable (DEPTH_TEST)", sc.depth_test_calls, sc.redundant_depth_test);
    push_item("enable/disable (CULL_FACE)", sc.cull_toggle_calls, sc.redundant_cull_toggles);
    push_item("colorMask", sc.color_mask_calls, sc.redundant_color_mask);
    push_item("useProgram", sc.use_program_calls, sc.redundant_use_program);
    push_item("bindTexture", sc.texture_bind_calls, sc.redundant_texture_binds);
    push_item("bindFramebuffer", sc.fbo_bind_calls, sc.redundant_fbo_binds);
    push_item("bindVertexArray", sc.vao_bind_calls, sc.redundant_vao_binds);

    let total_calls: usize = items.iter().map(|i| i.total_calls).sum();
    let total_redundant: usize = items.iter().map(|i| i.redundant_calls).sum();
    let overall_pct = if total_calls > 0 { (total_redundant as f64 / total_calls as f64) * 100.0 } else { 0.0 };

    let mut warnings = Vec::new();
    if sc.redundant_depth_mask > 0 {
        let pct = (sc.redundant_depth_mask as f64 / sc.depth_mask_calls.max(1) as f64) * 100.0;
        warnings.push(format!(
            "{} redundant depthMask() calls ({:.1}% of depthMask). Babylon.js state cache bypass: calling depthMask with existing value stalls the WebGL pipeline.",
            sc.redundant_depth_mask, pct
        ));
    }
    if sc.redundant_blend_toggles > 0 {
        warnings.push(format!(
            "{} redundant BLEND toggle calls. Avoid enabling/disabling GL_BLEND when already in that state.",
            sc.redundant_blend_toggles
        ));
    }
    if sc.redundant_depth_test > 0 {
        warnings.push(format!("{} redundant DEPTH_TEST toggle calls.", sc.redundant_depth_test));
    }
    if sc.redundant_use_program > 0 {
        warnings.push(format!(
            "{} redundant useProgram() calls. Redundant shader program switches waste CPU cycles.",
            sc.redundant_use_program
        ));
    }

    StateAnalysis {
        total_state_calls: total_calls,
        total_redundant_calls: total_redundant,
        overall_redundancy_pct: overall_pct,
        items,
        opaque_draws: capture.summary.opaque_draws,
        blended_draws: capture.summary.blended_draws,
        warnings,
    }
}
