mod template;

use self::template::html_template;

use crate::analysis::ComprehensiveAnalysis;
use crate::model::ParsedCapture;
use crate::report::{format_bytes, format_num};

pub fn generate_html_report(capture: &ParsedCapture, analysis: &ComprehensiveAnalysis) -> String {
    let title = "gprof — WebGL Frame Profile Dashboard";
    let renderer = capture.context.as_ref().and_then(|c| c.renderer.as_deref()).unwrap_or("Unknown GPU");
    let version = capture.context.as_ref().and_then(|c| c.version).unwrap_or(2);
    let total_vram = format_bytes(capture.summary.total_texture_bytes + analysis.fbos.total_offscreen_vram_bytes + analysis.buffers.total_buffer_memory_bytes);

    let mut lint_cards = String::new();
    for r in &analysis.lint.rules {
        let bg_color = match r.severity {
            crate::analysis::LintSeverity::Critical => "rgba(239, 68, 68, 0.15)",
            crate::analysis::LintSeverity::High => "rgba(249, 115, 22, 0.15)",
            crate::analysis::LintSeverity::Medium => "rgba(234, 179, 8, 0.15)",
            crate::analysis::LintSeverity::Low => "rgba(59, 130, 246, 0.15)",
        };
        let border_color = match r.severity {
            crate::analysis::LintSeverity::Critical => "#ef4444",
            crate::analysis::LintSeverity::High => "#f97316",
            crate::analysis::LintSeverity::Medium => "#eab308",
            crate::analysis::LintSeverity::Low => "#3b82f6",
        };

        lint_cards.push_str(&format!(
            r#"<div class="card" style="border-left: 4px solid {}; background: {}; margin-bottom: 16px;">
                <div style="display:flex; justify-content:space-between; align-items:center;">
                    <h3 style="margin:0;">{} [{}] {}</h3>
                    <span class="badge" style="background:{};">{}</span>
                </div>
                <p style="margin:8px 0; color:#cbd5e1;"><strong>Impact:</strong> {}</p>
                <p style="margin:8px 0; color:#94a3b8;">{}</p>
                <div style="margin-top:10px;">
                    <div style="font-size:12px; color:#38bdf8; font-weight:600; margin-bottom:4px;">TypeScript / Babylon.js Fix:</div>
                    <pre><code>{}</code></pre>
                </div>
            </div>"#,
            border_color,
            bg_color,
            r.severity.emoji(),
            r.id,
            r.title,
            border_color,
            r.severity.as_str(),
            r.impact,
            r.description,
            r.code_fix
        ));
    }

    let mut passes_rows = String::new();
    for p in &capture.passes {
        let (vw, vh) = (p.viewport_width.unwrap_or(0), p.viewport_height.unwrap_or(0));
        let res_str = if vw > 0 && vh > 0 { format!("{}x{}", vw, vh) } else { "—".to_string() };
        passes_rows.push_str(&format!(
            "<tr><td>#{}</td><td><code>{}</code></td><td><span class=\"badge-tag\">{}</span></td><td>{}</td><td><strong>{}</strong></td><td>{}</td><td>{}</td></tr>",
            p.pass_index,
            p.fbo_name,
            p.pass_type,
            res_str,
            p.draw_count,
            format_num(p.triangles),
            format_bytes(p.vram_bytes)
        ));
    }

    let mut textures_rows = String::new();
    for t in capture.textures.iter().take(50) {
        let unames = if t.uniform_names.is_empty() { "—".to_string() } else { t.uniform_names.join(", ") };
        let pot_badge = if t.is_pot {
            "<span style='color:#10b981;'>POT</span>"
        } else {
            "<span style='color:#f59e0b;'>NPOT</span>"
        };
        textures_rows.push_str(&format!(
            "<tr><td>#{}</td><td>{}x{}</td><td>{}</td><td><strong>{}</strong></td><td>{}</td><td>{}</td><td>{}</td><td><small>{}</small></td></tr>",
            t.id,
            t.width,
            t.height,
            pot_badge,
            format_bytes(t.length),
            t.internal_format_name,
            t.type_name,
            t.draw_count,
            unames
        ));
    }

    let mut draws_rows = String::new();
    for d in analysis.draws.heaviest_draws.iter().take(30) {
        let pos = d.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        draws_rows.push_str(&format!(
            "<tr><td>#{}</td><td><strong>{}</strong></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            d.index,
            format_num(d.triangles),
            d.shader_name,
            d.fbo_name,
            format_bytes(d.vertex_buffer_bytes),
            pos
        ));
    }

    let mut uniform_type_rows = String::new();
    for t in &analysis.uniforms.redundant_by_type {
        uniform_type_rows.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td><td><strong>{}</strong></td><td>{:.1}%</td><td>{}</td></tr>",
            t.type_name,
            format_num(t.total),
            format_num(t.redundant),
            t.redundant_pct,
            format_bytes(t.redundant_bytes)
        ));
    }

    let mut overdraw_rows = String::new();
    for p in &analysis.overdraw.pass_overdraw {
        overdraw_rows.push_str(&format!(
            "<tr><td>#{}</td><td><code>{}</code></td><td>{}x{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><strong>{:.1}x</strong></td><td>{}</td></tr>",
            p.pass_index,
            p.fbo_name,
            p.resolution.0,
            p.resolution.1,
            p.draw_count,
            p.quad_draws,
            format_num(p.blended_triangles),
            format_num(p.estimated_shaded_pixels),
            p.estimated_fillrate_multiplier,
            p.fillrate_pressure
        ));
    }

    let mut scene_tree_rows = String::new();
    for node in analysis.tree.nodes.iter().take(40) {
        let pos_str = node.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        let passes_str = node.passes_used.iter().map(|p| format!("#{}", p)).collect::<Vec<_>>().join(", ");
        let shaders_str = node.shaders.iter().map(|s| s.replace("fragment:", "").replace("vertex:", "")).collect::<Vec<_>>().join(", ");
        scene_tree_rows.push_str(&format!(
            "<tr><td>#{}</td><td><strong>{}</strong></td><td><span class=\"badge-tag\">{}</span></td><td>{}</td><td>{}</td><td><strong>{}</strong></td><td>{}</td><td><code>{}</code></td></tr>",
            node.id,
            node.name,
            node.category,
            pos_str,
            node.draw_count,
            format_num(node.total_triangles),
            passes_str,
            shaders_str
        ));
    }

    format!(html_template!(),
        title = title,
        version = version,
        renderer = renderer,
        total_cmd = format_num(capture.summary.total_commands),
        total_vram = total_vram,
        draw_calls = capture.summary.draw_calls,
        opaque_draws = capture.summary.opaque_draws,
        blended_draws = capture.summary.blended_draws,
        triangles = format_num(capture.summary.triangles),
        points = format_num(capture.summary.points),
        overdraw_factor = analysis.overdraw.overdraw_factor,
        overdraw_color = if analysis.overdraw.overdraw_factor >= 3.5 {
            "var(--danger)"
        } else if analysis.overdraw.overdraw_factor >= 2.0 {
            "var(--warning)"
        } else {
            "var(--success)"
        },
        shaded_pixels = format_num(analysis.overdraw.total_shaded_pixels_estimate),
        bb_w = analysis.overdraw.backbuffer_resolution.0,
        bb_h = analysis.overdraw.backbuffer_resolution.1,
        quad_passes = analysis.overdraw.fullscreen_quad_passes,
        uniform_calls = analysis.uniforms.total_calls,
        uniform_red_calls = analysis.uniforms.redundant_calls,
        uniform_red_pct = analysis.uniforms.redundancy_pct,
        uniform_red_matrices = analysis.uniforms.redundant_matrix_calls,
        avg_uniforms_per_draw = analysis.uniforms.avg_uniforms_per_draw,
        scene_nodes_count = analysis.tree.total_nodes,
        lint_issues = analysis.lint.total_issues,
        critical_issues = analysis.lint.critical_count,
        high_issues = analysis.lint.high_count,
        textures_count = capture.textures.len(),
        passes_rows = passes_rows,
        scene_tree_rows = scene_tree_rows,
        lint_cards = lint_cards,
        overdraw_rows = overdraw_rows,
        uniform_type_rows = uniform_type_rows,
        total_switches = analysis.sorting.total_switches,
        min_switches = analysis.sorting.min_possible_switches,
        ping_pong = analysis.sorting.ping_pong_switches,
        opaque_after_blend = analysis.sorting.total_opaque_after_blended,
        sorting_efficiency = analysis.sorting.overall_efficiency_pct,
        textures_rows = textures_rows,
        draws_rows = draws_rows
    )
}
