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
