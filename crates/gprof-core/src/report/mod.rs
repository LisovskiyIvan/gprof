use std::fmt::Write as _;

macro_rules! w {
    ($out:ident, $($arg:tt)*) => {
        let _ = write!($out, $($arg)*);
    };
}

mod tables;

pub use tables::*;


use crate::analysis::ComprehensiveAnalysis;
use crate::model::ParsedCapture;

pub fn format_bytes(bytes: usize) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else if mb >= 1.0 {
        format!("{:.1} MB", mb)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn format_full_report(capture: &ParsedCapture, analysis: &ComprehensiveAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# gprof — SpectorJS WebGL Frame Profile Report\n\n");

    out.push_str("> [!NOTE]\n");
    out.push_str("> **Profiler Methodology & Confidence Tiers:**\n");
    out.push_str("> Findings in this report are categorized by evidence certainty:\n");
    out.push_str("> - **[Observed Fact]** — Directly extracted from the SpectorJS API stream (exact call counts, buffer sizes, formats).\n");
    out.push_str("> - **[Calculated Estimate]** — Algorithmic approximation with stated assumptions (VRAM consumption, screen-space coverage, instancing savings).\n");
    out.push_str("> - **[Hypothesis / Candidate]** — Architecture or optimization opportunity requiring developer verification (occlusion masking necessity, render target scaling, half-float precision).\n");
    out.push_str(">\n");
    out.push_str("> *GPU execution bottleneck not established; findings below are candidates for inspection, not proven causes of low FPS without hardware GPU timestamps.*\n\n");

    // Executive Summary
    out.push_str("## 1. Executive Summary\n\n");
    out.push_str("| Metric | Value | Detail |\n");
    out.push_str("|---|---|---|\n");
    w!(out, "| **Total WebGL Commands** | **{}** | In single captured frame |\n", format_num(capture.summary.total_commands));
    w!(out, 
        "| **Draw Calls** | **{}** | {} elements, {} instanced elements, {} arrays, {} instanced arrays |\n",
        capture.summary.draw_calls,
        capture.summary.draw_elements,
        capture.summary.draw_elements_instanced,
        capture.summary.draw_arrays,
        capture.summary.draw_arrays_instanced
    );
    w!(out, "| **Triangles** | **{}** | Geometry rendered across all passes |\n", format_num(capture.summary.triangles));
    w!(out, "| **Points / Particles** | **{}** | {} active GPUParticleSystems |\n", format_num(capture.summary.points), capture.summary.particle_systems_count);
    w!(out, "| **Programs (Shaders)** | **{}** | {} useProgram switches ({} driver redundant) |\n", capture.summary.unique_programs, capture.summary.use_program_calls, capture.summary.redundant_use_program);
    w!(out, "| **Texture Memory (VRAM)** | **{}** | {} textures in GPU memory |\n", format_bytes(capture.summary.total_texture_bytes), capture.summary.total_textures_count);
    w!(out, "| **Texture Binds** | **{}** | activeTexture / bindTexture calls |\n", capture.summary.texture_bind_calls);
    w!(out, "| **Framebuffers (FBOs)** | **{}** | {} bindFramebuffer calls |\n", capture.summary.active_fbo_count, capture.summary.fbo_bind_calls);
    w!(out, 
        "| **Dynamic Uploads** | **{}** | {} texImage2D, {} bufferSubData ({} duplicate) |\n",
        capture.summary.dynamic_uploads_count + capture.summary.buffer_subdata_calls,
        capture.summary.dynamic_uploads_count,
        capture.summary.buffer_subdata_calls,
        capture.summary.duplicate_buffer_uploads
    );
    w!(out, 
        "| **Redundant State Calls** | **{}** | Redundant depthMask, blend, depthTest, useProgram |\n",
        capture.state_changes.total_redundant_calls
    );
    w!(out, 
        "| **Index Buffer 32-bit** | **{}** | Draw calls using UNSIGNED_INT >65k verts (vs {} 16-bit) |\n",
        capture.summary.index_32bit_draws,
        capture.summary.index_16bit_draws
    );
    out.push('\n');

    // Glow Layer Audit
    out.push_str("## 2. GlowLayer Occlusion & Emission Audit\n\n");
    w!(out, "- Total Glow Passes: {} draw calls, {} triangles\n", analysis.glow.total_draws, format_num(analysis.glow.total_triangles));
    w!(out, "- **Non-Emissive Occluder Geometry:** {} draw calls, {} triangles (**{:.1}%** of glow geometry)\n", analysis.glow.black_occluder_draws, format_num(analysis.glow.black_occluder_triangles), analysis.glow.occluder_triangle_percentage);
    w!(out, "- **Active Emissive Glowing Geometry:** {} draw calls, {} triangles\n\n", analysis.glow.active_glowing_draws, format_num(analysis.glow.active_glowing_triangles));

    if !analysis.glow.top_black_occluders.is_empty() {
        out.push_str("### Heaviest Non-Emissive Occluder Meshes in GlowLayer:\n\n");
        out.push_str("| Draw # | Command # | Triangles | Buffer (KB) | World Position (X, Y, Z) |\n");
        out.push_str("|---|---|---|---|---|\n");
        for m in &analysis.glow.top_black_occluders {
            let pos_str = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            w!(out, 
                "| #{} | #{} | **{}** | {:.1} KB | {} |\n",
                m.draw_index,
                m.command_id,
                format_num(m.triangles),
                m.vertex_buffer_bytes as f64 / 1024.0,
                pos_str
            );
        }
        out.push('\n');
    }

    // Textures Memory
    out.push_str("## 3. Texture Memory Breakdown\n\n");
    out.push_str("| Category | Count | Total VRAM | Sample IDs |\n");
    out.push_str("|---|---|---|---|\n");
    for b in &analysis.textures.buckets {
        let sample_str = b.sample_ids.iter().map(|id| format!("#{}", id)).collect::<Vec<_>>().join(", ");
        w!(out, "| {} | {} | **{}** | {} |\n", b.name, b.count, format_bytes(b.total_bytes), sample_str);
    }
    out.push('\n');

    // Render Passes
    out.push_str("## 4. Render Passes Sequence\n\n");
    out.push_str("| Pass # | Target FBO | Pass Type | Draws | Triangles | Key Shaders |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for p in &analysis.passes.passes {
        if p.draw_count > 0 {
            let shaders_summary = p.shaders.iter().map(|s| s.replace("vertex:", "")).collect::<Vec<_>>().join(", ");
            w!(out, 
                "| {} | {} | {} | {} | {} | {} |\n",
                p.pass_index,
                p.fbo_name,
                p.pass_type,
                p.draw_count,
                format_num(p.triangles),
                shaders_summary
            );
        }
    }
    out.push('\n');

    // Heaviest Draw Calls
    out.push_str("## 5. Heaviest Draw Calls (Top Geometry)\n\n");
    out.push_str("| Draw # | Triangles | Target FBO | Shader | World Pos | Description |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for d in analysis.draws.heaviest_draws.iter().take(15) {
        let pos_str = d.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        let short_desc = if d.text.len() > 40 { format!("{}...", &d.text[..40]) } else { d.text.clone() };
        w!(out, 
            "| #{} | **{}** | {} | {} | {} | {} |\n",
            d.index,
            format_num(d.triangles),
            d.fbo_name,
            d.shader_name.replace("vertex:", ""),
            pos_str,
            short_desc
        );
    }
    out.push('\n');

    // Shaders & PBR Fragmentation
    out.push_str("## 6. Shaders & Programs\n\n");
    w!(out, "- Total WebGL Programs: **{}**\n", analysis.shaders.total_programs);
    w!(out, "- Total `useProgram` calls: **{}** (redundant: {})\n", analysis.shaders.use_program_calls, analysis.shaders.redundant_use_program_calls);
    w!(out, "- PBR Shader Variants: **{}**\n", analysis.shaders.pbr_variants_count);
    w!(out, "- Common PBR Defines: **{}**\n\n", analysis.shaders.pbr_common_defines_count);

    if !analysis.shaders.fragmentation_reasons.is_empty() {
        out.push_str("### Causes of PBR Shader Permutations:\n\n");
        for r in &analysis.shaders.fragmentation_reasons {
            w!(out, "- {}\n", r);
        }
        out.push('\n');
    }

    // GPU Particles
    out.push_str("## 7. GPU Particles (Transform Feedback)\n\n");
    w!(out, "- Active Systems: **{}**\n", analysis.particles.systems_count);
    w!(out, "- Total Particles: **{}** points\n", format_num(analysis.particles.total_particles));
    w!(out, "- Total Quad Triangles Rendered: **{}**\n\n", format_num(analysis.particles.total_quad_triangles));

    if !analysis.particles.systems.is_empty() {
        out.push_str("| System # | Update Draw # | Render Draw # | Particles | Quad Tris |\n");
        out.push_str("|---|---|---|---|---|\n");
        for (i, sys) in analysis.particles.systems.iter().enumerate() {
            let rend_str = sys.render_draw_index.map(|idx| format!("#{}", idx)).unwrap_or_else(|| "—".to_string());
            w!(out, 
                "| {} | #{} | {} | **{}** | {} |\n",
                i + 1,
                sys.update_draw_index,
                rend_str,
                format_num(sys.particle_count),
                format_num(sys.quads_rendered)
            );
        }
        out.push('\n');
    }

    // Instancing Candidates
    if analysis.instancing.total_potential_saved_draws > 0 {
        out.push_str("## 8. Instancing & Batching Candidates\n\n");
        out.push_str("| Buffer Size (KB) | Tris/Instance | Calls | Total Tris | Potential Saved Calls | Sample Draws |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for c in analysis.instancing.candidates.iter().take(10) {
            let draws_sample = c.draw_indices.iter().take(4).map(|i| format!("#{}", i)).collect::<Vec<_>>().join(", ");
            w!(out, 
                "| {:.1} KB | {} | **{}** | {} | **-{} calls** | {} |\n",
                c.position_buffer_bytes as f64 / 1024.0,
                format_num(c.triangles_per_instance),
                c.draw_count,
                format_num(c.total_triangles),
                c.potential_saved_draws,
                draws_sample
            );
        }
        out.push('\n');
    }

    // WebGL State Thrashing & Redundancy
    out.push_str("## 9. WebGL State Thrashing & Redundancy\n\n");
    w!(out, "- Total WebGL State Calls: **{}**\n", format_num(analysis.states.total_state_calls));
    w!(out, "- Total Redundant Calls: **{}** (**{:.1}%** redundant)\n", format_num(analysis.states.total_redundant_calls), analysis.states.overall_redundancy_pct);
    w!(out, "- Draw Call Modes: **{}** opaque, **{}** blended (alpha)\n\n", analysis.states.opaque_draws, analysis.states.blended_draws);

    out.push_str("| WebGL State Call | Total Calls | Redundant Calls | Redundant % |\n");
    out.push_str("|---|---|---|---|\n");
    for s in &analysis.states.items {
        if s.total_calls > 0 {
            w!(out, "| `{}` | {} | **{}** | **{:.1}%** |\n", s.name, s.total_calls, s.redundant_calls, s.redundant_pct);
        }
    }
    out.push('\n');

    // Dynamic Buffers & Geometry
    out.push_str("## 10. Geometry Buffers & Dynamic Uploads\n\n");
    w!(out, "- Geometry Buffer Memory (VRAM): **{}**\n", format_bytes(analysis.buffers.total_buffer_memory_bytes));
    w!(out, "- Index Buffer Bitness: **{}** 16-bit draws, **{}** 32-bit draws\n", analysis.buffers.index_16bit_draws, analysis.buffers.index_32bit_draws);
    w!(out, "- Dynamic Buffer Uploads: **{}** calls ({} duplicate), **{}** uploaded/frame\n\n",
        analysis.buffers.total_dynamic_uploads,
        analysis.buffers.duplicate_dynamic_uploads,
        format_bytes(analysis.buffers.total_upload_bytes)
    );

    if !analysis.buffers.meshes_32bit.is_empty() {
        out.push_str("### Draw Calls Using 32-Bit Index Buffers (UNSIGNED_INT >65k vertices):\n\n");
        out.push_str("| Draw # | Command # | Triangles | Target FBO | Shader | World Pos |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for m in &analysis.buffers.meshes_32bit {
            let pos_str = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            w!(out, 
                "| #{} | #{} | **{}** | {} | {} | {} |\n",
                m.draw_index,
                m.command_id,
                format_num(m.triangles),
                m.fbo_name,
                m.shader_name,
                pos_str
            );
        }
        out.push('\n');
    }

    if !analysis.buffers.uploads.is_empty() {
        out.push_str("### Dynamic Buffer Uploads (bufferSubData):\n\n");
        out.push_str("| Command # | Target | Offset | Size | Duplicate | Origin Stack Trace |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for u in &analysis.buffers.uploads {
            let trace = u.stack_trace.first().map(|s| s.as_str()).unwrap_or("—");
            w!(out, 
                "| #{} | {} | {} | {} | {} | `{}` |\n",
                u.command_index,
                u.target_name,
                u.offset,
                format_bytes(u.size_bytes),
                if u.is_duplicate { "⚠️ Yes" } else { "No" },
                trace
            );
        }
        out.push('\n');
    }

    // Texture Compression & NPOT
    out.push_str("## 11. Texture Compression & NPOT Audit\n\n");
    w!(out, "- Uncompressed Color Textures: **{}** ({})\n", analysis.compression.uncompressed_color_count, format_bytes(analysis.compression.uncompressed_color_bytes));
    w!(out, "- Theoretical KTX2 (ASTC / BC7) Upper Bound Savings: **{}** (~75% color VRAM)\n", format_bytes(analysis.compression.estimated_ktx2_savings_bytes));
    w!(out, "- GPU Animation / VAT Data Textures: **{}** (Experimental Half-Float savings: {})\n", format_bytes(analysis.compression.float_vat_bytes), format_bytes(analysis.compression.estimated_half_float_savings_bytes));
    w!(out, "- Non-Power-of-Two (NPOT) Textures: **{}** (Supported in WebGL 2; check CLAMP_TO_EDGE for WebGL 1 fallback)\n\n", analysis.compression.npot_count);

    // Render Targets (FBO)
    out.push_str("## 12. Render Targets (FBO) VRAM Footprint\n\n");
    w!(out, "- Total Active FBOs: **{}**\n", analysis.fbos.total_fbos_count);
    w!(out, "- Total Offscreen FBO VRAM: **{}**\n", format_bytes(analysis.fbos.total_offscreen_vram_bytes));
    if let Some((w, h)) = analysis.fbos.backbuffer_resolution {
        w!(out, "- Canvas DrawingBuffer Resolution: **{}x{}**\n\n", w, h);
    } else {
        out.push('\n');
    }
    out.push_str("| FBO Name | Resolution | Dimension Source | Estimated VRAM | Passes | Draws | Tris | Oversized |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for f in &analysis.fbos.fbos {
        let res = if f.width > 0 && f.height > 0 { format!("{}x{}", f.width, f.height) } else { "—".to_string() };
        w!(out, 
            "| `{}` | {} | {} | **{}** | {} | {} | {} | {} |\n",
            f.fbo_name,
            res,
            f.dimension_source,
            format_bytes(f.vram_bytes),
            f.pass_count,
            f.draw_count,
            format_num(f.triangles),
            if f.is_oversized { "⚠️ Candidate" } else { "No" }
        );
    }
    out.push('\n');

    // Material & Shader Sorting
    out.push_str("## 13. Material & Shader Sorting Efficiency\n\n");
    w!(out, "- Consecutive Shader Switches: **{}** (Phase-Constrained Min: **{}**)\n", analysis.sorting.total_switches, analysis.sorting.min_possible_switches);
    w!(out, "- Reducible Shader Switches: **{}**\n", analysis.sorting.reducible_switches);
    w!(out, "- Driver Redundant useProgram Calls: **{}**\n", analysis.sorting.api_redundant_use_program);
    w!(out, "- Ping-Pong Switches (A -> B -> A): **{}**\n", analysis.sorting.ping_pong_switches);
    w!(out, "- Overall Sorting Efficiency: **{:.1}%**\n", analysis.sorting.overall_efficiency_pct);
    if analysis.sorting.total_opaque_after_blended > 0 {
        w!(out, "- ℹ️ **Opaque Draws After Blend:** **{}** (verify if UI/overlay or 3D geometry)\n", analysis.sorting.total_opaque_after_blended);
    }
    out.push('\n');

    // Detected WebGL / Babylon Patterns Summary
    out.push_str("## 14. Detected WebGL & Engine Patterns Summary\n\n");
    w!(out, 
        "Identified **{}** patterns (🔴 {} Critical, 🟠 {} High, 🟡 {} Medium, ℹ️ {} Low).\n\n",
        analysis.lint.total_issues,
        analysis.lint.critical_count,
        analysis.lint.high_count,
        analysis.lint.medium_count,
        analysis.lint.low_count
    );
    out.push_str("| Rule ID | Severity | Pattern / Title | Detected Metric |\n");
    out.push_str("|---|---|---|---|\n");
    for r in &analysis.lint.rules {
        w!(out, "| `{}` | {} | {} | {} |\n", r.id, r.severity.as_str(), r.title, r.impact);
    }
    out.push('\n');

    // Uniforms Overhead
    out.push_str("## 15. Uniforms Overhead & Redundancy Audit\n\n");
    w!(out, "- Total gl.uniform* API Calls: **{}** (Average per draw: **{:.2}**)\n", format_num(analysis.uniforms.total_calls), analysis.uniforms.avg_api_calls_per_draw);
    w!(out, "- Active Uniform Declarations per Draw: **{:.1}** (Max: **{}**)\n", analysis.uniforms.avg_active_uniforms_per_draw, analysis.uniforms.max_uniforms_in_draw);
    w!(out, "- Redundant API Calls: **{}** (**{:.1}%** redundant)\n", format_num(analysis.uniforms.redundant_calls), analysis.uniforms.redundancy_pct);
    w!(out, "- CPU-to-Driver Payload: **{}** (Redundant: **{}**)\n", format_bytes(analysis.uniforms.total_bytes), format_bytes(analysis.uniforms.redundant_bytes));
    w!(out, "- Matrix Calls: **{}** (Redundant: **{}**)\n\n", analysis.uniforms.matrix_calls, analysis.uniforms.redundant_matrix_calls);

    // Screen-Space Coverage & Estimated Fillrate
    out.push_str("## 16. Estimated Screen-Space Coverage & Fillrate Pressure\n\n");
    w!(out, "- Estimated Screen Coverage Multiplier: **{:.1}x** (Coverage Level: **{}**)\n", analysis.overdraw.estimated_screenspace_coverage, analysis.overdraw.risk_level);
    w!(out, "- Coverage Breakdown: **{:.1}x** post-process quads, **{:.1}x** scene geometry & blending\n", analysis.overdraw.postprocess_quad_coverage, analysis.overdraw.geometry_coverage);
    w!(out, "- Estimated Shaded Pixel Equivalents: **{}** (Target Canvas: {}x{})\n",
        format_num(analysis.overdraw.total_shaded_pixels_estimate),
        analysis.overdraw.backbuffer_resolution.0,
        analysis.overdraw.backbuffer_resolution.1
    );
    w!(out, "- Full-Screen Post-Process Quads: **{}**\n", analysis.overdraw.fullscreen_quad_passes);
    w!(out, "- Blended Transparent Draws: **{}** ({} triangles)\n", analysis.overdraw.blended_draw_calls, format_num(analysis.overdraw.blended_triangles));
    w!(out, "> [!NOTE]\n> {}\n\n", analysis.overdraw.methodology_disclaimer);

    // Scene Tree
    out.push_str("## 17. Scene Tree & Mesh Hierarchy Reconstruction\n\n");
    w!(out, "- Reconstructed Logical Scene Nodes: **{}** (Source names unavailable in SpectorJS)\n", analysis.tree.total_nodes);
    out.push_str("| Category | Node Count | Total Draws | Total Triangles |\n");
    out.push_str("|---|---|---|---|\n");
    for c in &analysis.tree.categories_summary {
        w!(out, "| **{}** | {} | {} | **{}** |\n", c.category, c.node_count, c.draw_count, format_num(c.triangle_count));
    }
    out.push('\n');

    out
}

