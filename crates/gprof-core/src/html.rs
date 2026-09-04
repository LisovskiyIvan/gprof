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
        .header h1 {{ margin: 0; font-size: 24px; font-weight: 700; color: #fff; }}
        .header .subtitle {{ color: var(--text-muted); font-size: 14px; margin-top: 4px; }}
        .container {{ max-width: 1400px; margin: 0 auto; padding: 24px 32px; }}
        .kpi-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
            gap: 16px;
            margin-bottom: 24px;
        }}
        .kpi-card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px 20px;
        }}
        .kpi-title {{ font-size: 13px; color: var(--text-muted); text-transform: uppercase; font-weight: 600; letter-spacing: 0.5px; }}
        .kpi-value {{ font-size: 28px; font-weight: 700; color: #fff; margin: 6px 0; }}
        .kpi-detail {{ font-size: 12px; color: var(--text-muted); }}
        .tabs {{
            display: flex;
            gap: 8px;
            border-bottom: 1px solid var(--border);
            margin-bottom: 24px;
        }}
        .tab-btn {{
            background: none;
            border: none;
            color: var(--text-muted);
            padding: 10px 16px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            border-bottom: 2px solid transparent;
            transition: all 0.2s;
        }}
        .tab-btn.active {{ color: var(--primary); border-bottom-color: var(--primary); }}
        .tab-content {{ display: none; }}
        .tab-content.active {{ display: block; }}
        .card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 20px;
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
        th {{ color: var(--text-muted); font-weight: 600; background: rgba(0,0,0,0.2); }}
        tr:hover td {{ background: rgba(255,255,255,0.02); }}
        code {{
            background: #1e293b;
            color: #38bdf8;
            padding: 2px 6px;
            border-radius: 4px;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            font-size: 12px;
        }}
        pre {{
            background: #0f172a;
            border: 1px solid var(--border);
            padding: 12px;
            border-radius: 6px;
            overflow-x: auto;
            margin: 8px 0 0 0;
        }}
        pre code {{ background: none; color: #a5f3fc; padding: 0; }}
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
                <div class="kpi-title">Texture VRAM</div>
                <div class="kpi-value">{tex_vram}</div>
                <div class="kpi-detail">Save up to {save_vram} via KTX2</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Glow Occluder Waste</div>
                <div class="kpi-value" style="color:var(--danger);">{glow_waste_pct:.1}%</div>
                <div class="kpi-detail">{glow_black_tris} wasted triangles</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">State Redundancy</div>
                <div class="kpi-value" style="color:var(--warning);">{redundant_pct:.1}%</div>
                <div class="kpi-detail">{redundant_calls} redundant driver calls</div>
            </div>
            <div class="kpi-card">
                <div class="kpi-title">Linter Issues</div>
                <div class="kpi-value">{lint_issues}</div>
                <div class="kpi-detail">🔴 {critical_issues} Critical, 🟠 {high_issues} High</div>
            </div>
        </div>

        <div class="tabs">
            <button class="tab-btn active" onclick="openTab('overview')">Overview & Pipeline</button>
            <button class="tab-btn" onclick="openTab('lint')">Babylon.js Linter ({lint_issues})</button>
            <button class="tab-btn" onclick="openTab('sorting')">Material & State Sorting</button>
            <button class="tab-btn" onclick="openTab('textures')">Textures & VRAM ({textures_count})</button>
            <button class="tab-btn" onclick="openTab('draws')">Draw Calls Explorer</button>
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

        <!-- TAB: LINTER -->
        <div id="lint" class="tab-content">
            <div class="card">
                <h2 style="margin-top:0;">Babylon.js Code Optimization Rules & Fixes</h2>
                <p style="color:var(--text-muted); font-size:14px;">Automated engine linter checking for common WebGL and Babylon.js performance pitfalls.</p>
                {lint_cards}
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
        tex_vram = format_bytes(capture.summary.total_texture_bytes),
        save_vram = format_bytes(analysis.compression.estimated_ktx2_savings_bytes),
        glow_waste_pct = analysis.glow.wasted_triangle_percentage,
        glow_black_tris = format_num(analysis.glow.black_occluder_triangles),
        redundant_pct = analysis.states.overall_redundancy_pct,
        redundant_calls = analysis.states.total_redundant_calls,
        lint_issues = analysis.lint.total_issues,
        critical_issues = analysis.lint.critical_count,
        high_issues = analysis.lint.high_count,
        textures_count = capture.textures.len(),
        passes_rows = passes_rows,
        lint_cards = lint_cards,
        total_switches = analysis.sorting.total_switches,
        min_switches = analysis.sorting.min_possible_switches,
        ping_pong = analysis.sorting.ping_pong_switches,
        opaque_after_blend = analysis.sorting.total_opaque_after_blended,
        sorting_efficiency = analysis.sorting.overall_efficiency_pct,
        textures_rows = textures_rows,
        draws_rows = draws_rows
    )
}
