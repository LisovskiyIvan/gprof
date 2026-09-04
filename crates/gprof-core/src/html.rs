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
        let pot_badge = if t.is_pot { "<span style='color:#10b981;'>POT</span>" } else { "<span style='color:#f59e0b;'>NPOT</span>" };
        textures_rows.push_str(&format!(
            "<tr><td>#{}</td><td>{}x{}</td><td>{}</td><td><strong>{}</strong></td><td>{}</td><td>{}</td><td>{}</td><td><small>{}</small></td></tr>",
            t.id, t.width, t.height, pot_badge, format_bytes(t.length), t.internal_format_name, t.type_name, t.draw_count, unames
        ));
    }

    let mut draws_rows = String::new();
    for d in analysis.draws.heaviest_draws.iter().take(30) {
        let pos = d.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        draws_rows.push_str(&format!(
            "<tr><td>#{}</td><td><strong>{}</strong></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            d.index, format_num(d.triangles), d.shader_name, d.fbo_name, format_bytes(d.vertex_buffer_bytes), pos
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

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        :root {{
            --bg: #0b0f19;
            --surface: #111827;
            --border: #1f2937;
            --text: #f3f4f6;
            --text-muted: #9ca3af;
            --primary: #38bdf8;
            --accent: #ec4899;
            --success: #10b981;
            --warning: #f59e0b;
            --danger: #ef4444;
        }}
        body {{
            margin: 0;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background: var(--bg);
            color: var(--text);
            line-height: 1.5;
        }}
        .header {{
            background: var(--surface);
            border-bottom: 1px solid var(--border);
            padding: 20px 32px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .header h1 {{
            margin: 0;
            font-size: 24px;
            font-weight: 700;
            letter-spacing: -0.5px;
        }}
        .subtitle {{
            color: var(--text-muted);
            font-size: 14px;
            margin-top: 4px;
        }}
        .container {{
            max-width: 1440px;
            margin: 0 auto;
            padding: 32px;
        }}
        .kpi-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 16px;
            margin-bottom: 32px;
        }}
        .kpi-card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 20px;
        }}
        .kpi-title {{
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-muted);
            margin-bottom: 8px;
        }}
        .kpi-value {{
            font-size: 28px;
            font-weight: 700;
            color: var(--primary);
        }}
        .kpi-detail {{
            font-size: 12px;
            color: var(--text-muted);
            margin-top: 4px;
        }}
        .tabs {{
            display: flex;
            gap: 8px;
            border-bottom: 1px solid var(--border);
            margin-bottom: 24px;
            overflow-x: auto;
        }}
        .tab-btn {{
            background: none;
            border: none;
            color: var(--text-muted);
            padding: 12px 18px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            border-bottom: 2px solid transparent;
            transition: all 0.2s;
            white-space: nowrap;
        }}
        .tab-btn:hover {{
            color: var(--text);
        }}
        .tab-btn.active {{
            color: var(--primary);
            border-bottom-color: var(--primary);
        }}
        .tab-content {{
            display: none;
        }}
        .tab-content.active {{
            display: block;
        }}
        .card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 24px;
            margin-bottom: 24px;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            font-size: 13px;
        }}
        th, td {{
            text-align: left;
            padding: 10px 12px;
            border-bottom: 1px solid var(--border);
        }}
        th {{
            color: var(--text-muted);
            font-weight: 600;
            text-transform: uppercase;
            font-size: 11px;
            letter-spacing: 0.5px;
        }}
        tr:hover td {{
            background: rgba(255, 255, 255, 0.02);
        }}
        code {{
            background: rgba(0, 0, 0, 0.3);
            padding: 2px 6px;
            border-radius: 4px;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            font-size: 12px;
            color: var(--primary);
        }}
        pre {{
            background: #05070d;
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 12px;
            overflow-x: auto;
            margin: 4px 0 0 0;
        }}
        pre code {{
            background: none;
            padding: 0;
            color: #38bdf8;
            font-size: 12px;
        }}
        .badge {{
            display: inline-block;
            padding: 2px 8px;
            border-radius: 9999px;
            font-size: 11px;
            font-weight: 700;
            color: #fff;
        }}
        .badge-tag {{
            background: #1e293b;
            color: #94a3b8;
            padding: 2px 6px;
            border-radius: 4px;
            font-size: 11px;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div>
            <h1>⚡ gprof — WebGL Frame Profile</h1>
            <div class="subtitle">WebGL {version} • {renderer} • Total Commands: {total_cmd}</div>
        </div>
        <div>
            <span class="badge" style="background: #0284c7; font-size:13px; padding:6px 14px;">Total VRAM: {total_vram}</span>
        </div>
    </div>

    <div class="container">
        <div class="kpi-grid">
            <div class="kpi-card">
                <div class="kpi-title">Draw Calls</div>
                <div class="kpi-value">{draw_calls}</div>
                <div class="kpi-detail">{opaque_draws} opaque, {blended_draws} blended</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Triangles</div>
                <div class="kpi-value">{triangles}</div>
                <div class="kpi-detail">{points} particles / points</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Frame Overdraw</div>
                <div class="kpi-value" style="color:{overdraw_color};">{overdraw_factor:.1}x</div>
                <div class="kpi-detail">{shaded_pixels} shaded pixels</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Uniform Redundancy</div>
                <div class="kpi-value" style="color:var(--warning);">{uniform_red_pct:.1}%</div>
                <div class="kpi-detail">{uniform_red_calls} redundant uploads</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Sorting Efficiency</div>
                <div class="kpi-value">{sorting_efficiency:.1}%</div>
                <div class="kpi-detail">{ping_pong} ping-pong switches</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Linter Issues</div>
                <div class="kpi-value">{lint_issues}</div>
                <div class="kpi-detail">🔴 {critical_issues} Critical, 🟠 {high_issues} High</div>
            </div>
        </div>

        <div class="tabs">
            <button class="tab-btn active" onclick="openTab('overview')">Pipeline & Passes</button>
            <button class="tab-btn" onclick="openTab('tree')">Scene Tree & Nodes ({scene_nodes_count})</button>
            <button class="tab-btn" onclick="openTab('lint')">Babylon.js Linter ({lint_issues})</button>
            <button class="tab-btn" onclick="openTab('overdraw')">Overdraw & Fillrate</button>
            <button class="tab-btn" onclick="openTab('uniforms')">Uniforms ({uniform_calls})</button>
            <button class="tab-btn" onclick="openTab('sorting')">Material Sorting</button>
            <button class="tab-btn" onclick="openTab('textures')">Textures & VRAM ({textures_count})</button>
            <button class="tab-btn" onclick="openTab('draws')">Draw Calls</button>
        </div>

        <!-- TAB: OVERVIEW -->
        <div id="overview" class="tab-content active">
            <div class="card">
                <h2 style="margin-top:0;">Render Passes Sequence</h2>
                <table>
                    <thead>
                        <tr>
                            <th>Pass #</th>
                            <th>Target FBO</th>
                            <th>Pass Type</th>
                            <th>Viewport</th>
                            <th>Draw Calls</th>
                            <th>Triangles</th>
                            <th>Estimated VRAM</th>
                        </tr>
                    </thead>
                    <tbody>
                        {passes_rows}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- TAB: SCENE TREE -->
        <div id="tree" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Reconstructed Scene Tree & Object Hierarchy</h2>
                <p style="color:var(--text-muted); font-size:14px;">Draw calls clustered by physical scene entities across all passes.</p>
                <table>
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Node Name</th>
                            <th>Category</th>
                            <th>World Position</th>
                            <th>Draws</th>
                            <th>Triangles</th>
                            <th>Passes</th>
                            <th>Shaders</th>
                        </tr>
                    </thead>
                    <tbody>
                        {scene_tree_rows}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- TAB: LINTER -->
        <div id="lint" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Babylon.js Code Optimization Rules & Fixes</h2>
                <p style="color:var(--text-muted); font-size:14px;">Automated engine linter checking for common WebGL and Babylon.js performance pitfalls.</p>
                {lint_cards}
            </div>
        </div>

        <!-- TAB: OVERDRAW -->
        <div id="overdraw" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Screen-Space Overdraw & Fillrate Analysis</h2>
                <div style="display:flex; gap:32px; margin-bottom:16px;">
                    <div><strong>Overdraw Factor:</strong> <span style="color:{overdraw_color}; font-weight:700;">{overdraw_factor:.1}x</span></div>
                    <div><strong>Target Resolution:</strong> {bb_w}x{bb_h}</div>
                    <div><strong>Full-screen Quads:</strong> {quad_passes}</div>
                    <div><strong>Blended Draws:</strong> {blended_draws}</div>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Pass #</th>
                            <th>Target FBO</th>
                            <th>Viewport</th>
                            <th>Draws</th>
                            <th>Quads</th>
                            <th>Blended Tris</th>
                            <th>Shaded Pixels Est.</th>
                            <th>Multiplier</th>
                            <th>Risk</th>
                        </tr>
                    </thead>
                    <tbody>
                        {overdraw_rows}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- TAB: UNIFORMS -->
        <div id="uniforms" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Uniforms Overhead & Redundancy Audit</h2>
                <div style="display:flex; gap:32px; margin-bottom:16px;">
                    <div><strong>Total Uniform Calls:</strong> {uniform_calls}</div>
                    <div><strong>Redundant Calls:</strong> <span style="color:var(--warning); font-weight:700;">{uniform_red_calls} ({uniform_red_pct:.1}%)</span></div>
                    <div><strong>Redundant Matrices:</strong> {uniform_red_matrices}</div>
                    <div><strong>Avg per Draw:</strong> {avg_uniforms_per_draw:.1}</div>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Command Type</th>
                            <th>Total Calls</th>
                            <th>Redundant Calls</th>
                            <th>Redundant %</th>
                            <th>Wasted Traffic</th>
                        </tr>
                    </thead>
                    <tbody>
                        {uniform_type_rows}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- TAB: SORTING -->
        <div id="sorting" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Material & Shader Sorting Efficiency</h2>
                <div style="display:flex; gap:32px; margin-bottom:16px;">
                    <div><strong>Shader Switches:</strong> {total_switches} (Min: {min_switches})</div>
                    <div><strong>Ping-Pong Switches (A->B->A):</strong> <span style="color:var(--warning); font-weight:700;">{ping_pong}</span></div>
                    <div><strong>Opaque After Blend:</strong> <span style="color:var(--danger); font-weight:700;">{opaque_after_blend}</span></div>
                    <div><strong>Sorting Efficiency:</strong> <strong>{sorting_efficiency:.1}%</strong></div>
                </div>
            </div>
        </div>

        <!-- TAB: TEXTURES -->
        <div id="textures" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Textures VRAM Footprint</h2>
                <table>
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Dimensions</th>
                            <th>POT</th>
                            <th>VRAM Size</th>
                            <th>Format</th>
                            <th>Type</th>
                            <th>Draws</th>
                            <th>Sample Uniforms</th>
                        </tr>
                    </thead>
                    <tbody>
                        {textures_rows}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- TAB: DRAWS -->
        <div id="draws" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Top Heaviest Draw Calls (By Triangles)</h2>
                <table>
                    <thead>
                        <tr>
                            <th>Draw #</th>
                            <th>Triangles</th>
                            <th>Shader</th>
                            <th>Target FBO</th>
                            <th>Vertex Buffer</th>
                            <th>World Position</th>
                        </tr>
                    </thead>
                    <tbody>
                        {draws_rows}
                    </tbody>
                </table>
            </div>
        </div>
    </div>

    <script>
        function openTab(tabId) {{
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            document.getElementById(tabId).classList.add('active');
            event.currentTarget.classList.add('active');
        }}
    </script>
</body>
</html>"#,
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
        overdraw_color = if analysis.overdraw.overdraw_factor >= 3.5 { "var(--danger)" } else if analysis.overdraw.overdraw_factor >= 2.0 { "var(--warning)" } else { "var(--success)" },
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
