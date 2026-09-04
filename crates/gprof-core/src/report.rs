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
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn format_full_report(capture: &ParsedCapture, analysis: &ComprehensiveAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# gprof — SpectorJS WebGL Frame Profile Report\n\n");

    // Executive Summary
    out.push_str("## 1. Executive Summary\n\n");
    out.push_str("| Metric | Value | Detail |\n");
    out.push_str("|---|---|---|\n");
    out.push_str(&format!("| **Total WebGL Commands** | **{}** | In single captured frame |\n", format_num(capture.summary.total_commands)));
    out.push_str(&format!(
        "| **Draw Calls** | **{}** | {} elements, {} instanced elements, {} arrays, {} instanced arrays |\n",
        capture.summary.draw_calls,
        capture.summary.draw_elements,
        capture.summary.draw_elements_instanced,
        capture.summary.draw_arrays,
        capture.summary.draw_arrays_instanced
    ));
    out.push_str(&format!("| **Triangles** | **{}** | Geometry rendered across all passes |\n", format_num(capture.summary.triangles)));
    out.push_str(&format!("| **Points / Particles** | **{}** | {} active GPUParticleSystems |\n", format_num(capture.summary.points), capture.summary.particle_systems_count));
    out.push_str(&format!("| **Programs (Shaders)** | **{}** | {} useProgram switches ({} redundant) |\n", capture.summary.unique_programs, capture.summary.use_program_calls, capture.summary.redundant_use_program));
    out.push_str(&format!("| **Texture Memory (VRAM)** | **{}** | {} textures in GPU memory |\n", format_bytes(capture.summary.total_texture_bytes), capture.summary.total_textures_count));
    out.push_str(&format!("| **Texture Binds** | **{}** | activeTexture / bindTexture calls |\n", capture.summary.texture_bind_calls));
    out.push_str(&format!("| **Framebuffers (FBOs)** | **{}** | {} bindFramebuffer calls |\n", capture.summary.active_fbo_count, capture.summary.fbo_bind_calls));
    out.push_str(&format!(
        "| **Dynamic Uploads** | **{}** | {} texImage2D, {} bufferSubData ({} duplicate) |\n",
        capture.summary.dynamic_uploads_count + capture.summary.buffer_subdata_calls,
        capture.summary.dynamic_uploads_count,
        capture.summary.buffer_subdata_calls,
        capture.summary.duplicate_buffer_uploads
    ));
    out.push_str(&format!(
        "| **Redundant State Calls** | **{}** | Redundant depthMask, blend, depthTest, useProgram |\n",
        capture.state_changes.total_redundant_calls
    ));
    out.push_str(&format!(
        "| **Index Buffer 32-bit** | **{}** | Draw calls using UNSIGNED_INT >65k verts (vs {} 16-bit) |\n",
        capture.summary.index_32bit_draws,
        capture.summary.index_16bit_draws
    ));
    out.push_str("\n");

    // Critical Warnings / Quick Wins
    out.push_str("## 2. Critical Bottlenecks & Quick Wins\n\n");

    if analysis.glow.black_occluder_draws > 0 {
        out.push_str(&format!(
            "- 🔴 **GlowLayer Occluder Waste:** **{}** of **{}** glow draw calls ({:.1}% of glow triangles, **{}** tris) are drawn with `[0,0,0,1]` as black occluders! Switching to whitelist (`addIncludedOnlyMesh`) immediately removes {} draw calls and {} triangles.\n",
            analysis.glow.black_occluder_draws,
            analysis.glow.total_draws,
            analysis.glow.wasted_triangle_percentage,
            format_num(analysis.glow.black_occluder_triangles),
            analysis.glow.black_occluder_draws,
            format_num(analysis.glow.black_occluder_triangles)
        ));
    }

    for w in &analysis.textures.warnings {
        if w.kind == "OVERSIZED_8K" {
            out.push_str(&format!("- 🔴 **Oversized Texture #{} ({}x{}):** {}\n", w.texture_id, w.width, w.height, w.message));
        }
    }

    if let Some(w) = &analysis.uploads.warning {
        out.push_str(&format!("- 🟡 **Dynamic Texture Uploads:** {}\n", w));
    }

    for w in &analysis.states.warnings {
        out.push_str(&format!("- 🟡 **State Thrashing:** {}\n", w));
    }

    for w in &analysis.buffers.warnings {
        out.push_str(&format!("- 🟡 **Buffer / Geometry:** {}\n", w));
    }

    if analysis.instancing.total_potential_saved_draws > 0 {
        out.push_str(&format!(
            "- 🟡 **Instancing Potential:** Found **{}** groups of duplicate geometry drawn in separate calls. Merging into Thin Instances could eliminate up to **{}** draw calls per frame.\n",
            analysis.instancing.total_candidates_groups,
            analysis.instancing.total_potential_saved_draws
        ));
    }

    if let Some(w) = &analysis.particles.warning {
        out.push_str(&format!("- 🟡 **Particle Overdraw Risk:** {}\n", w));
    }

    if analysis.shaders.pbr_variants_count > 5 {
        out.push_str(&format!(
            "- 🟡 **PBR Shader Cache Fragmentation:** **{}** distinct PBR variants active in single frame. Top causes: {}.\n",
            analysis.shaders.pbr_variants_count,
            analysis.shaders.fragmentation_reasons.join("; ")
        ));
    }
    out.push_str("\n");

    // Glow Layer Audit
    out.push_str("## 3. GlowLayer Audit\n\n");
    out.push_str(&format!("- Total Glow Passes: {} draw calls, {} triangles\n", analysis.glow.total_draws, format_num(analysis.glow.total_triangles)));
    out.push_str(&format!("- **Black Occluders:** {} draw calls, {} triangles (**{:.1}%**)\n", analysis.glow.black_occluder_draws, format_num(analysis.glow.black_occluder_triangles), analysis.glow.wasted_triangle_percentage));
    out.push_str(&format!("- **Active Glowing:** {} draw calls, {} triangles\n\n", analysis.glow.active_glowing_draws, format_num(analysis.glow.active_glowing_triangles)));

    if !analysis.glow.top_black_occluders.is_empty() {
        out.push_str("### Heaviest Black Occluders Rendered in GlowLayer:\n\n");
        out.push_str("| Draw # | Command # | Triangles | Buffer (KB) | World Position (X, Y, Z) |\n");
        out.push_str("|---|---|---|---|---|\n");
        for m in &analysis.glow.top_black_occluders {
            let pos_str = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| #{} | #{} | **{}** | {:.1} KB | {} |\n",
                m.draw_index,
                m.command_id,
                format_num(m.triangles),
                m.vertex_buffer_bytes as f64 / 1024.0,
                pos_str
            ));
        }
        out.push_str("\n");
    }

    // Textures Memory
    out.push_str("## 4. Texture Memory Breakdown\n\n");
    out.push_str("| Category | Count | Total VRAM | Sample IDs |\n");
    out.push_str("|---|---|---|---|\n");
    for b in &analysis.textures.buckets {
        let sample_str = b.sample_ids.iter().map(|id| format!("#{}", id)).collect::<Vec<_>>().join(", ");
        out.push_str(&format!("| {} | {} | **{}** | {} |\n", b.name, b.count, format_bytes(b.total_bytes), sample_str));
    }
    out.push_str("\n");

    // Render Passes
    out.push_str("## 5. Render Passes Sequence\n\n");
    out.push_str("| Pass # | Target FBO | Pass Type | Draws | Triangles | Key Shaders |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for p in &analysis.passes.passes {
        if p.draw_count > 0 {
            let shaders_summary = p.shaders.iter().map(|s| s.replace("vertex:", "")).collect::<Vec<_>>().join(", ");
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                p.pass_index,
                p.fbo_name,
                p.pass_type,
                p.draw_count,
                format_num(p.triangles),
                shaders_summary
            ));
        }
    }
    out.push_str("\n");

    // Heaviest Draw Calls
    out.push_str("## 6. Heaviest Draw Calls (Top Geometry)\n\n");
    out.push_str("| Draw # | Triangles | Target FBO | Shader | World Pos | Description |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for d in analysis.draws.heaviest_draws.iter().take(15) {
        let pos_str = d.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        let short_desc = if d.text.len() > 40 { format!("{}...", &d.text[..40]) } else { d.text.clone() };
        out.push_str(&format!(
            "| #{} | **{}** | {} | {} | {} | {} |\n",
            d.index,
            format_num(d.triangles),
            d.fbo_name,
            d.shader_name.replace("vertex:", ""),
            pos_str,
            short_desc
        ));
    }
    out.push_str("\n");

    // Shaders & PBR Fragmentation
    out.push_str("## 7. Shaders & Programs\n\n");
    out.push_str(&format!("- Total WebGL Programs: **{}**\n", analysis.shaders.total_programs));
    out.push_str(&format!("- Total `useProgram` calls: **{}** (redundant: {})\n", analysis.shaders.use_program_calls, analysis.shaders.redundant_use_program_calls));
    out.push_str(&format!("- PBR Shader Variants: **{}**\n", analysis.shaders.pbr_variants_count));
    out.push_str(&format!("- Common PBR Defines: **{}**\n\n", analysis.shaders.pbr_common_defines_count));

    if !analysis.shaders.fragmentation_reasons.is_empty() {
        out.push_str("### Causes of PBR Shader Permutations:\n\n");
        for r in &analysis.shaders.fragmentation_reasons {
            out.push_str(&format!("- {}\n", r));
        }
        out.push_str("\n");
    }

    // GPU Particles
    out.push_str("## 8. GPU Particles (Transform Feedback)\n\n");
    out.push_str(&format!("- Active Systems: **{}**\n", analysis.particles.systems_count));
    out.push_str(&format!("- Total Particles: **{}** points\n", format_num(analysis.particles.total_particles)));
    out.push_str(&format!("- Total Quad Triangles Rendered: **{}**\n\n", format_num(analysis.particles.total_quad_triangles)));

    if !analysis.particles.systems.is_empty() {
        out.push_str("| System # | Update Draw # | Render Draw # | Particles | Quad Tris |\n");
        out.push_str("|---|---|---|---|---|\n");
        for (i, sys) in analysis.particles.systems.iter().enumerate() {
            let rend_str = sys.render_draw_index.map(|idx| format!("#{}", idx)).unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| {} | #{} | {} | **{}** | {} |\n",
                i + 1,
                sys.update_draw_index,
                rend_str,
                format_num(sys.particle_count),
                format_num(sys.quads_rendered)
            ));
        }
        out.push_str("\n");
    }

    // Instancing Candidates
    if analysis.instancing.total_potential_saved_draws > 0 {
        out.push_str("## 9. Instancing & Batching Candidates\n\n");
        out.push_str(&format!("> [!TIP]\n> {}\n\n", analysis.instancing.recommendation));
        out.push_str("| Buffer Size (KB) | Tris/Instance | Calls | Total Tris | Potential Saved Calls | Sample Draws |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for c in analysis.instancing.candidates.iter().take(10) {
            let draws_sample = c.draw_indices.iter().take(4).map(|i| format!("#{}", i)).collect::<Vec<_>>().join(", ");
            out.push_str(&format!(
                "| {:.1} KB | {} | **{}** | {} | **-{} calls** | {} |\n",
                c.position_buffer_bytes as f64 / 1024.0,
                format_num(c.triangles_per_instance),
                c.draw_count,
                format_num(c.total_triangles),
                c.potential_saved_draws,
                draws_sample
            ));
        }
        out.push_str("\n");
    }

    // WebGL State Thrashing & Redundancy
    out.push_str("## 10. WebGL State Thrashing & Redundancy\n\n");
    out.push_str(&format!("- Total WebGL State Calls: **{}**\n", format_num(analysis.states.total_state_calls)));
    out.push_str(&format!("- Total Redundant Calls: **{}** (**{:.1}%** redundant)\n", format_num(analysis.states.total_redundant_calls), analysis.states.overall_redundancy_pct));
    out.push_str(&format!("- Draw Call Modes: **{}** opaque, **{}** blended (alpha)\n\n", analysis.states.opaque_draws, analysis.states.blended_draws));

    out.push_str("| WebGL State Call | Total Calls | Redundant Calls | Redundant % |\n");
    out.push_str("|---|---|---|---|\n");
    for s in &analysis.states.items {
        if s.total_calls > 0 {
            out.push_str(&format!("| `{}` | {} | **{}** | **{:.1}%** |\n", s.name, s.total_calls, s.redundant_calls, s.redundant_pct));
        }
    }
    out.push_str("\n");

    // Dynamic Buffers & Geometry
    out.push_str("## 11. Geometry Buffers & Dynamic Uploads\n\n");
    out.push_str(&format!("- Geometry Buffer Memory (VRAM): **{}**\n", format_bytes(analysis.buffers.total_buffer_memory_bytes)));
    out.push_str(&format!("- Index Buffer Bitness: **{}** 16-bit draws, **{}** 32-bit draws\n", analysis.buffers.index_16bit_draws, analysis.buffers.index_32bit_draws));
    out.push_str(&format!("- Dynamic Buffer Uploads: **{}** calls ({} duplicate), **{}** uploaded/frame\n\n",
        analysis.buffers.total_dynamic_uploads,
        analysis.buffers.duplicate_dynamic_uploads,
        format_bytes(analysis.buffers.total_upload_bytes)
    ));

    if !analysis.buffers.meshes_32bit.is_empty() {
        out.push_str("### Draw Calls Using 32-Bit Index Buffers (UNSIGNED_INT >65k vertices):\n\n");
        out.push_str("| Draw # | Command # | Triangles | Target FBO | Shader | World Pos |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for m in &analysis.buffers.meshes_32bit {
            let pos_str = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| #{} | #{} | **{}** | {} | {} | {} |\n",
                m.draw_index,
                m.command_id,
                format_num(m.triangles),
                m.fbo_name,
                m.shader_name,
                pos_str
            ));
        }
        out.push_str("\n");
    }

    if !analysis.buffers.uploads.is_empty() {
        out.push_str("### Dynamic Buffer Uploads (bufferSubData):\n\n");
        out.push_str("| Command # | Target | Offset | Size | Duplicate | Origin Stack Trace |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for u in &analysis.buffers.uploads {
            let trace = u.stack_trace.first().map(|s| s.as_str()).unwrap_or("—");
            out.push_str(&format!(
                "| #{} | {} | {} | {} | {} | `{}` |\n",
                u.command_index,
                u.target_name,
                u.offset,
                format_bytes(u.size_bytes),
                if u.is_duplicate { "⚠️ Yes" } else { "No" },
                trace
            ));
        }
        out.push_str("\n");
    }

    // Texture Compression & NPOT
    out.push_str("## 12. Texture Compression & NPOT Audit\n\n");
    out.push_str(&format!("- Uncompressed Color Textures: **{}** ({})\n", analysis.compression.uncompressed_color_count, format_bytes(analysis.compression.uncompressed_color_bytes)));
    out.push_str(&format!("- Potential KTX2 / BC7 / ASTC Savings: **{}** (-75%)\n", format_bytes(analysis.compression.estimated_ktx2_savings_bytes)));
    out.push_str(&format!("- Float / VAT Textures: **{}** (Half-Float savings: {})\n", format_bytes(analysis.compression.float_vat_bytes), format_bytes(analysis.compression.estimated_half_float_savings_bytes)));
    out.push_str(&format!("- Non-Power-of-Two (NPOT) Textures: **{}**\n\n", analysis.compression.npot_count));

    // Render Targets (FBO)
    out.push_str("## 13. Render Targets (FBO) VRAM Footprint\n\n");
    out.push_str(&format!("- Total Active FBOs: **{}**\n", analysis.fbos.total_fbos_count));
    out.push_str(&format!("- Total Offscreen FBO VRAM: **{}**\n", format_bytes(analysis.fbos.total_offscreen_vram_bytes)));
    if let Some((w, h)) = analysis.fbos.backbuffer_resolution {
        out.push_str(&format!("- Backbuffer Resolution: **{}x{}**\n\n", w, h));
    } else {
        out.push_str("\n");
    }
    out.push_str("| FBO Name | Viewport | Estimated VRAM | Passes | Draws | Tris | Oversized |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for f in &analysis.fbos.fbos {
        let res = if f.width > 0 && f.height > 0 { format!("{}x{}", f.width, f.height) } else { "—".to_string() };
        out.push_str(&format!(
            "| `{}` | {} | **{}** | {} | {} | {} | {} |\n",
            f.fbo_name,
            res,
            format_bytes(f.vram_bytes),
            f.pass_count,
            f.draw_count,
            format_num(f.triangles),
            if f.is_oversized { "⚠️ Yes" } else { "No" }
        ));
    }
    out.push_str("\n");

    // Material & Shader Sorting
    out.push_str("## 14. Material & Shader Sorting Efficiency\n\n");
    out.push_str(&format!("- Consecutive Shader Switches: **{}** (Min possible: **{}**)\n", analysis.sorting.total_switches, analysis.sorting.min_possible_switches));
    out.push_str(&format!("- Redundant Shader Switches: **{}**\n", analysis.sorting.redundant_switches));
    out.push_str(&format!("- Ping-Pong Switches (A -> B -> A): **{}**\n", analysis.sorting.ping_pong_switches));
    out.push_str(&format!("- Overall Sorting Efficiency: **{:.1}%**\n", analysis.sorting.overall_efficiency_pct));
    if analysis.sorting.total_opaque_after_blended > 0 {
        out.push_str(&format!("- ⚠️ **Opaque Draws After Blend:** **{}** (breaks Early-Z rejection)\n", analysis.sorting.total_opaque_after_blended));
    }
    out.push_str("\n");

    // Babylon.js Linter Summary
    out.push_str("## 15. Babylon.js Performance Lint Summary\n\n");
    out.push_str(&format!(
        "Found **{}** issues (🔴 {} Critical, 🟠 {} High, 🟡 {} Medium, ℹ️ {} Low). Run `gprof lint` for complete TypeScript code fixes.\n\n",
        analysis.lint.total_issues,
        analysis.lint.critical_count,
        analysis.lint.high_count,
        analysis.lint.medium_count,
        analysis.lint.low_count
    ));
    for r in &analysis.lint.rules {
        out.push_str(&format!("- {} **[{}]** {}: {}\n", r.severity.emoji(), r.id, r.title, r.impact));
    }
    out.push_str("\n");

    // Uniforms Overhead
    out.push_str("## 16. Uniforms Overhead & Redundancy Audit\n\n");
    out.push_str(&format!("- Total Uniform Calls: **{}**\n", format_num(analysis.uniforms.total_calls)));
    out.push_str(&format!("- Redundant Calls: **{}** (**{:.1}%** redundant)\n", format_num(analysis.uniforms.redundant_calls), analysis.uniforms.redundancy_pct));
    out.push_str(&format!("- Matrix Calls: **{}** (Redundant: **{}**)\n", analysis.uniforms.matrix_calls, analysis.uniforms.redundant_matrix_calls));
    out.push_str(&format!("- Uniform Data Volume: **{}** (Redundant: **{}**)\n", format_bytes(analysis.uniforms.total_bytes), format_bytes(analysis.uniforms.redundant_bytes)));
    out.push_str(&format!("- Average Uniforms per Draw: **{:.1}** (Max: **{}**)\n\n", analysis.uniforms.avg_uniforms_per_draw, analysis.uniforms.max_uniforms_in_draw));

    // Overdraw & Fillrate
    out.push_str("## 17. Screen-Space Overdraw & Fillrate Analysis\n\n");
    out.push_str(&format!("- Estimated Overdraw Factor: **{:.1}x** (Risk Level: **{}**)\n", analysis.overdraw.overdraw_factor, analysis.overdraw.risk_level));
    out.push_str(&format!("- Estimated Shaded Pixels: **{}** (Target Backbuffer: {}x{})\n",
        format_num(analysis.overdraw.total_shaded_pixels_estimate),
        analysis.overdraw.backbuffer_resolution.0,
        analysis.overdraw.backbuffer_resolution.1
    ));
    out.push_str(&format!("- Full-Screen Post-Process Quads: **{}**\n", analysis.overdraw.fullscreen_quad_passes));
    out.push_str(&format!("- Blended Transparent Draws: **{}** ({} triangles)\n\n", analysis.overdraw.blended_draw_calls, format_num(analysis.overdraw.blended_triangles)));

    // Scene Tree
    out.push_str("## 18. Scene Tree & Mesh Hierarchy Reconstruction\n\n");
    out.push_str(&format!("- Reconstructed Logical Scene Nodes: **{}**\n", analysis.tree.total_nodes));
    out.push_str("| Category | Node Count | Total Draws | Total Triangles |\n");
    out.push_str("|---|---|---|---|\n");
    for c in &analysis.tree.categories_summary {
        out.push_str(&format!("| **{}** | {} | {} | **{}** |\n", c.category, c.node_count, c.draw_count, format_num(c.triangle_count)));
    }
    out.push_str("\n");

    out
}

pub fn format_states_table(analysis: &crate::analysis::StateAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# WebGL State Thrashing & Redundancy Audit\n\n");
    out.push_str(&format!("- Total WebGL State Calls: **{}**\n", format_num(analysis.total_state_calls)));
    out.push_str(&format!("- Total Redundant Calls: **{}** (**{:.1}%** redundant)\n", format_num(analysis.total_redundant_calls), analysis.overall_redundancy_pct));
    out.push_str(&format!("- Draw Call Modes: **{}** opaque, **{}** blended (alpha)\n\n", analysis.opaque_draws, analysis.blended_draws));

    if !analysis.warnings.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for w in &analysis.warnings {
            out.push_str(&format!("- ⚠️ {}\n", w));
        }
        out.push_str("\n");
    }

    out.push_str("## State Changes Breakdown\n\n");
    out.push_str("| WebGL State Call | Total Calls | Redundant Calls | Redundant % |\n");
    out.push_str("|---|---|---|---|\n");
    for s in &analysis.items {
        if s.total_calls > 0 {
            out.push_str(&format!("| `{}` | {} | **{}** | **{:.1}%** |\n", s.name, s.total_calls, s.redundant_calls, s.redundant_pct));
        }
    }
    out
}

pub fn format_buffers_table(analysis: &crate::analysis::BufferAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# Geometry Buffers & Dynamic Uploads Audit\n\n");
    out.push_str(&format!("- Geometry Buffer Memory (VRAM): **{}**\n", format_bytes(analysis.total_buffer_memory_bytes)));
    out.push_str(&format!("- Index Buffer Bitness: **{}** 16-bit draws, **{}** 32-bit draws\n", analysis.index_16bit_draws, analysis.index_32bit_draws));
    out.push_str(&format!("- Dynamic Buffer Uploads: **{}** calls ({} duplicate), **{}** uploaded/frame\n\n",
        analysis.total_dynamic_uploads,
        analysis.duplicate_dynamic_uploads,
        format_bytes(analysis.total_upload_bytes)
    ));

    if !analysis.warnings.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for w in &analysis.warnings {
            out.push_str(&format!("- ⚠️ {}\n", w));
        }
        out.push_str("\n");
    }

    if !analysis.meshes_32bit.is_empty() {
        out.push_str("## Draw Calls Using 32-Bit Index Buffers (UNSIGNED_INT >65k vertices)\n\n");
        out.push_str("| Draw # | Command # | Triangles | Target FBO | Shader | World Pos |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for m in &analysis.meshes_32bit {
            let pos_str = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "| #{} | #{} | **{}** | {} | {} | {} |\n",
                m.draw_index,
                m.command_id,
                format_num(m.triangles),
                m.fbo_name,
                m.shader_name,
                pos_str
            ));
        }
        out.push_str("\n");
    }

    if !analysis.uploads.is_empty() {
        out.push_str("## Dynamic Buffer Uploads (bufferSubData)\n\n");
        out.push_str("| Command # | Target | Offset | Size | Duplicate | Origin Stack Trace |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for u in &analysis.uploads {
            let trace = u.stack_trace.first().map(|s| s.as_str()).unwrap_or("—");
            out.push_str(&format!(
                "| #{} | {} | {} | {} | {} | `{}` |\n",
                u.command_index,
                u.target_name,
                u.offset,
                format_bytes(u.size_bytes),
                if u.is_duplicate { "⚠️ Yes" } else { "No" },
                trace
            ));
        }
        out.push_str("\n");
    }

    out
}

pub fn format_textures_table(textures: &[crate::model::TextureItem], top_n: usize) -> String {
    let mut out = String::new();
    out.push_str("# Textures Detailed Breakdown\n\n");
    out.push_str("| ID | Dimensions | Memory | Format / Internal | Type | Compressed | Draws | Uniforms |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for t in textures.iter().take(top_n) {
        let unames = if t.uniform_names.is_empty() { "—".to_string() } else { t.uniform_names.join(", ") };
        out.push_str(&format!(
            "| #{} | {}x{} | **{}** | {} / {} | {} | {} | {} | {} |\n",
            t.id,
            t.width,
            t.height,
            format_bytes(t.length),
            t.format_name,
            t.internal_format_name,
            t.type_name,
            if t.is_compressed { "Yes" } else { "No" },
            t.draw_count,
            unames
        ));
    }
    out
}

pub fn format_compression_table(analysis: &crate::analysis::CompressionAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# Texture Compression & NPOT Audit\n\n");
    out.push_str(&format!("- Total Textures: **{}**\n", analysis.total_textures));
    out.push_str(&format!("- Uncompressed Color Textures: **{}** ({})\n", analysis.uncompressed_color_count, format_bytes(analysis.uncompressed_color_bytes)));
    out.push_str(&format!("- Potential KTX2 (ASTC / BC7) Savings: **{}** (-75%)\n", format_bytes(analysis.estimated_ktx2_savings_bytes)));
    out.push_str(&format!("- Float / VAT Textures: **{}** (Half-Float savings: {})\n", format_bytes(analysis.float_vat_bytes), format_bytes(analysis.estimated_half_float_savings_bytes)));
    out.push_str(&format!("- Non-Power-of-Two (NPOT) Textures: **{}**\n\n", analysis.npot_count));

    if !analysis.warnings.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for w in &analysis.warnings {
            out.push_str(&format!("- ⚠️ {}\n", w));
        }
        out.push_str("\n");
    }

    if !analysis.npot_textures.is_empty() {
        out.push_str("## Non-Power-of-Two (NPOT) Textures List\n\n");
        out.push_str("| ID | Dimensions | Memory | Type | Draws |\n");
        out.push_str("|---|---|---|---|---|\n");
        for t in analysis.npot_textures.iter().take(25) {
            out.push_str(&format!(
                "| #{} | {}x{} | **{}** | {} | {} |\n",
                t.id,
                t.width,
                t.height,
                format_bytes(t.memory_bytes),
                if t.is_float { "Float Data" } else { "Color / Albedo" },
                t.draw_count
            ));
        }
        out.push_str("\n");
    }

    out
}

pub fn format_fbo_table(analysis: &crate::analysis::FboAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# Render Targets (FBO) VRAM Audit\n\n");
    out.push_str(&format!("- Total Active FBOs: **{}**\n", analysis.total_fbos_count));
    out.push_str(&format!("- Total Offscreen FBO VRAM: **{}**\n", format_bytes(analysis.total_offscreen_vram_bytes)));
    if let Some((w, h)) = analysis.backbuffer_resolution {
        out.push_str(&format!("- Backbuffer Resolution: **{}x{}**\n\n", w, h));
    } else {
        out.push_str("\n");
    }

    if !analysis.warnings.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for w in &analysis.warnings {
            out.push_str(&format!("- ⚠️ {}\n", w));
        }
        out.push_str("\n");
    }

    out.push_str("## FBO Allocation Breakdown\n\n");
    out.push_str("| FBO Name | Viewport | Estimated VRAM | Passes | Draws | Tris | Oversized |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for f in &analysis.fbos {
        let res = if f.width > 0 && f.height > 0 { format!("{}x{}", f.width, f.height) } else { "—".to_string() };
        out.push_str(&format!(
            "| `{}` | {} | **{}** | {} | {} | {} | {} |\n",
            f.fbo_name,
            res,
            format_bytes(f.vram_bytes),
            f.pass_count,
            f.draw_count,
            format_num(f.triangles),
            if f.is_oversized { "⚠️ Yes" } else { "No" }
        ));
    }

    out
}

pub fn format_sorting_table(analysis: &crate::analysis::SortingAnalysis) -> String {
    let mut out = String::new();
    out.push_str("# Material & Shader Sorting Efficiency Audit\n\n");
    out.push_str(&format!("- Consecutive Shader Switches: **{}** (Min possible: **{}**)\n", analysis.total_switches, analysis.min_possible_switches));
    out.push_str(&format!("- Redundant Shader Switches: **{}**\n", analysis.redundant_switches));
    out.push_str(&format!("- Ping-Pong Switches (A -> B -> A): **{}**\n", analysis.ping_pong_switches));
    out.push_str(&format!("- Overall Sorting Efficiency: **{:.1}%**\n", analysis.overall_efficiency_pct));
    if analysis.total_opaque_after_blended > 0 {
        out.push_str(&format!("- ⚠️ **Opaque Draws After Blend:** **{}** (breaks Early-Z rejection)\n", analysis.total_opaque_after_blended));
    }
    out.push_str("\n");

    if !analysis.warnings.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for w in &analysis.warnings {
            out.push_str(&format!("- ⚠️ {}\n", w));
        }
        out.push_str("\n");
    }

    out.push_str("## Pass-by-Pass Sorting Efficiency\n\n");
    out.push_str("| Pass # | FBO | Draws | Switches | Min Switches | Redundant | Ping-Pong | Efficiency | Opaque After Blend |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for p in &analysis.passes {
        out.push_str(&format!(
            "| #{} | `{}` | {} | {} | {} | **{}** | **{}** | **{:.1}%** | {} |\n",
            p.pass_index,
            p.fbo_name,
            p.draw_count,
            p.shader_switches,
            p.min_possible_switches,
            p.redundant_switches,
            p.ping_pong_switches,
            p.efficiency_pct,
            p.opaque_after_blended
        ));
    }

    out
}

pub fn format_uniforms_table(analysis: &crate::analysis::UniformStats) -> String {
    let mut out = String::new();
    out.push_str("# Uniforms Overhead & Redundancy Audit\n\n");
    out.push_str(&format!("- Total Uniform Calls: **{}**\n", format_num(analysis.total_calls)));
    out.push_str(&format!("- Redundant Uniform Calls: **{}** (**{:.1}%** redundant)\n", format_num(analysis.redundant_calls), analysis.redundancy_pct));
    out.push_str(&format!("- Uniform Data Transferred: **{}** (Redundant: **{}**)\n", format_bytes(analysis.total_bytes), format_bytes(analysis.redundant_bytes)));
    out.push_str(&format!("- Matrix Uniform Calls: **{}** (Redundant: **{}**)\n", analysis.matrix_calls, analysis.redundant_matrix_calls));
    out.push_str(&format!("- Average Uniforms per Draw Call: **{:.1}** (Max: **{}**)\n\n", analysis.avg_uniforms_per_draw, analysis.max_uniforms_in_draw));

    if !analysis.recommendations.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for r in &analysis.recommendations {
            out.push_str(&format!("- {}\n", r));
        }
        out.push_str("\n");
    }

    if !analysis.redundant_by_type.is_empty() {
        out.push_str("## Uniform Commands Breakdown\n\n");
        out.push_str("| Uniform Command | Total Calls | Redundant Calls | Redundant % | Wasted Bytes |\n");
        out.push_str("|---|---|---|---|---|\n");
        for t in &analysis.redundant_by_type {
            out.push_str(&format!(
                "| `{}` | {} | **{}** | **{:.1}%** | {} |\n",
                t.type_name,
                format_num(t.total),
                format_num(t.redundant),
                t.redundant_pct,
                format_bytes(t.redundant_bytes)
            ));
        }
        out.push_str("\n");
    }

    if !analysis.top_redundant_callers.is_empty() {
        out.push_str("## Top Redundant Uniform Callsites\n\n");
        out.push_str("| Callsite / Function | Total Calls | Redundant Calls | Redundant % |\n");
        out.push_str("|---|---|---|---|\n");
        for c in &analysis.top_redundant_callers {
            out.push_str(&format!(
                "| `{}` | {} | **{}** | **{:.1}%** |\n",
                c.caller,
                c.total_calls,
                c.redundant_calls,
                c.redundant_pct
            ));
        }
        out.push_str("\n");
    }

    out
}

pub fn format_overdraw_table(analysis: &crate::analysis::OverdrawAnalysisResult) -> String {
    let mut out = String::new();
    out.push_str("# Screen-Space Overdraw & Fillrate Analysis\n\n");
    out.push_str(&format!("- Target Backbuffer Resolution: **{}x{}** ({} pixels)\n",
        analysis.backbuffer_resolution.0,
        analysis.backbuffer_resolution.1,
        format_num(analysis.total_screen_pixels)
    ));
    out.push_str(&format!("- Estimated Frame Overdraw Factor: **{:.1}x** (Risk Level: **{}**)\n", analysis.overdraw_factor, analysis.risk_level));
    out.push_str(&format!("- Estimated Total Shaded Pixels: **{}** pixels/frame\n", format_num(analysis.total_shaded_pixels_estimate)));
    out.push_str(&format!("- Full-Screen Post-Process Quads: **{}**\n", analysis.fullscreen_quad_passes));
    out.push_str(&format!("- Blended / Transparent Draws: **{}** ({} triangles)\n", analysis.blended_draw_calls, format_num(analysis.blended_triangles)));
    if analysis.early_z_broken_draws > 0 {
        out.push_str(&format!("- ⚠️ Opaque Draws After Blend (Broken Early-Z): **{}**\n", analysis.early_z_broken_draws));
    }
    out.push_str("\n");

    if !analysis.recommendations.is_empty() {
        out.push_str("## Warnings & Opportunities\n\n");
        for r in &analysis.recommendations {
            out.push_str(&format!("- {}\n", r));
        }
        out.push_str("\n");
    }

    out.push_str("## Pass-by-Pass Fillrate & Overdraw Pressure\n\n");
    out.push_str("| Pass # | FBO Name | Viewport | Draws | Quads | Blended Tris | Shaded Pixels Est. | Fillrate Multiplier | Risk |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for p in &analysis.pass_overdraw {
        out.push_str(&format!(
            "| #{} | `{}` | {}x{} | {} | {} | {} | {} | **{:.1}x** | {} |\n",
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

    out
}

pub fn format_scene_tree_table(analysis: &crate::analysis::SceneTreeAnalysisResult, top: usize) -> String {
    let mut out = String::new();
    out.push_str("# Scene Tree & Mesh Hierarchy Reconstruction\n\n");
    out.push_str(&format!("- Reconstructed Scene Nodes: **{}**\n\n", analysis.total_nodes));

    out.push_str("## Scene Node Categories Breakdown\n\n");
    out.push_str("| Category | Node Count | Total Draws | Total Triangles |\n");
    out.push_str("|---|---|---|---|\n");
    for c in &analysis.categories_summary {
        out.push_str(&format!(
            "| **{}** | {} | {} | **{}** |\n",
            c.category,
            c.node_count,
            c.draw_count,
            format_num(c.triangle_count)
        ));
    }
    out.push_str("\n");

    out.push_str(&format!("## Top Scene Nodes (Top {})\n\n", top));
    out.push_str("| ID | Node Name | Category | World Pos | Draws | Triangles | Passes | Shaders |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for node in analysis.nodes.iter().take(top) {
        let pos_str = node.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        let passes_str = node.passes_used.iter().map(|p| format!("#{}", p)).collect::<Vec<_>>().join(", ");
        let shaders_str = node.shaders.iter().map(|s| s.replace("fragment:", "").replace("vertex:", "")).collect::<Vec<_>>().join(", ");
        out.push_str(&format!(
            "| #{} | **{}** | {} | {} | {} | **{}** | {} | `{}` |\n",
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

    out
}
