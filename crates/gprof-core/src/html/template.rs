macro_rules! html_template {
    () => {
        r#"<!DOCTYPE html>
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
</html>"#

    };
}

pub(crate) use html_template;
