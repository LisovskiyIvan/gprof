use gprof_core::{report, ComprehensiveAnalysis, ParsedCapture};

pub fn print_textures(capture: &ParsedCapture, analysis: &ComprehensiveAnalysis, top: usize) {
    println!(
        "# Textures Analysis (Total: {}, Memory: {})\n",
        capture.textures.len(),
        report::format_bytes(capture.summary.total_texture_bytes)
    );

    println!("## Memory Categories\n");
    println!("| Category | Count | Total VRAM | Sample IDs |");
    println!("|---|---|---|---|");
    for b in &analysis.textures.buckets {
        let sample = b.sample_ids.iter().map(|id| format!("#{}", id)).collect::<Vec<_>>().join(", ");
        println!("| {} | {} | **{}** | {} |", b.name, b.count, report::format_bytes(b.total_bytes), sample);
    }
    println!();

    println!("{}", report::format_textures_table(&capture.textures, top));
}

pub fn print_glow(analysis: &ComprehensiveAnalysis) {
    println!("# GlowLayer Non-Emissive Occluder Geometry Audit\n");
    println!(
        "- Total Glow Passes: **{}** draw calls, **{}** triangles",
        analysis.glow.total_draws,
        report::format_num(analysis.glow.total_triangles)
    );
    println!(
        "- 🟡 **Non-Emissive Occluders:** **{}** draw calls, **{}** triangles (**{:.1}%** of glow geometry)",
        analysis.glow.black_occluder_draws,
        report::format_num(analysis.glow.black_occluder_triangles),
        analysis.glow.occluder_triangle_percentage
    );
    println!(
        "- 🟢 **Active Emissive Glowing:** **{}** draw calls, **{}** triangles\n",
        analysis.glow.active_glowing_draws,
        report::format_num(analysis.glow.active_glowing_triangles)
    );

    if !analysis.glow.top_black_occluders.is_empty() {
        println!("## Heaviest Non-Emissive Occluder Meshes:\n");
        println!("| Draw # | Command # | Triangles | Buffer (KB) | World Position (X, Y, Z) |");
        println!("|---|---|---|---|---|");
        for m in &analysis.glow.top_black_occluders {
            let pos = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            println!(
                "| #{} | #{} | **{}** | {:.1} KB | {} |",
                m.draw_index,
                m.command_id,
                report::format_num(m.triangles),
                m.vertex_buffer_bytes as f64 / 1024.0,
                pos
            );
        }
    }
}

pub fn print_draws(analysis: &ComprehensiveAnalysis, top: usize) {
    println!("# Heaviest Draw Calls (Top {})\n", top);
    println!("- Total Draw Calls: **{}**", analysis.draws.total_draws);
    println!("- Total Triangles: **{}**", report::format_num(analysis.draws.total_triangles));
    println!("- Total Points: **{}**\n", report::format_num(analysis.draws.total_points));

    println!("| Draw # | Triangles | Target FBO | Shader | World Pos | Description |");
    println!("|---|---|---|---|---|---|");
    for d in analysis.draws.heaviest_draws.iter().take(top) {
        let pos = d.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
        let short_desc = if d.text.len() > 40 { format!("{}...", &d.text[..40]) } else { d.text.clone() };
        println!(
            "| #{} | **{}** | {} | {} | {} | {} |",
            d.index,
            report::format_num(d.triangles),
            d.fbo_name,
            d.shader_name.replace("vertex:", ""),
            pos,
            short_desc
        );
    }
}

pub fn print_passes(analysis: &ComprehensiveAnalysis) {
    println!("# Render Passes Sequence (Total: {})\n", analysis.passes.total_passes);
    println!("| Pass # | Target FBO | Pass Type | Draws | Triangles | Shaders |");
    println!("|---|---|---|---|---|---|");
    for p in &analysis.passes.passes {
        if p.draw_count > 0 {
            let shaders = p.shaders.iter().map(|s| s.replace("vertex:", "")).collect::<Vec<_>>().join(", ");
            println!(
                "| {} | {} | {} | {} | {} | {} |",
                p.pass_index,
                p.fbo_name,
                p.pass_type,
                p.draw_count,
                report::format_num(p.triangles),
                shaders
            );
        }
    }
}

pub fn print_shaders(analysis: &ComprehensiveAnalysis) {
    println!("# Shaders & Program Permutations\n");
    println!("- Total WebGL Programs: **{}**", analysis.shaders.total_programs);
    println!(
        "- `useProgram` calls: **{}** (redundant: {})",
        analysis.shaders.use_program_calls, analysis.shaders.redundant_use_program_calls
    );
    println!("- PBR Shader Variants in single frame: **{}**", analysis.shaders.pbr_variants_count);
    println!("- Common PBR Defines count: **{}**\n", analysis.shaders.pbr_common_defines_count);

    if !analysis.shaders.fragmentation_reasons.is_empty() {
        println!("## Why PBR Shaders Diverged:\n");
        for r in &analysis.shaders.fragmentation_reasons {
            println!("- ⚠️  {}", r);
        }
        println!();
    }

    println!("## PBR Variants by Draw Count:\n");
    println!("| Program ID | Draws | Unique Defines vs Common Baseline |");
    println!("|---|---|---|");
    for v in &analysis.shaders.pbr_variants {
        let diff_str = if v.unique_defines.is_empty() {
            "None (Baseline)".to_string()
        } else {
            v.unique_defines.join(", ")
        };
        println!("| #{} | {} | `{}` |", v.program_id, v.draw_count, diff_str);
    }
}

pub fn print_particles(analysis: &ComprehensiveAnalysis) {
    println!("# GPU Particles (Transform Feedback)\n");
    println!("- Active Systems: **{}**", analysis.particles.systems_count);
    println!("- Total Active Particles: **{}** points", report::format_num(analysis.particles.total_particles));
    println!("- Total Quad Triangles: **{}**\n", report::format_num(analysis.particles.total_quad_triangles));

    println!("| System # | Update Draw # | Render Draw # | Particle Count | Quad Triangles |");
    println!("|---|---|---|---|---|");
    for (i, sys) in analysis.particles.systems.iter().enumerate() {
        let rend = sys.render_draw_index.map(|idx| format!("#{}", idx)).unwrap_or_else(|| "—".to_string());
        println!(
            "| {} | #{} | {} | **{}** | {} |",
            i + 1,
            sys.update_draw_index,
            rend,
            report::format_num(sys.particle_count),
            report::format_num(sys.quads_rendered)
        );
    }
}

pub fn print_uploads(analysis: &ComprehensiveAnalysis) {
    println!("# Dynamic Texture Uploads\n");
    println!("- Total per-frame uploads: **{}**\n", analysis.uploads.total_uploads);

    println!("| # | Command | Target | Dimensions | Stack Trace |");
    println!("|---|---|---|---|---|");
    for (i, u) in analysis.uploads.uploads.iter().enumerate() {
        let st = u.stack_trace.iter().take(2).cloned().collect::<Vec<_>>().join(" → ");
        let st_str = if st.is_empty() { "—".to_string() } else { st };
        println!("| {} | {} | {} | {}x{} | `{}` |", i + 1, u.name, u.target, u.width, u.height, st_str);
    }
}

pub fn print_instancing(analysis: &ComprehensiveAnalysis) {
    println!("# Instancing & Batching Audit\n");
    println!("- Potential Saved Draw Calls: **-{}** calls/frame", analysis.instancing.total_potential_saved_draws);
    println!("- Candidate Groups: **{}**\n", analysis.instancing.total_candidates_groups);

    println!("| Buffer (KB) | Tris/Instance | Calls | Total Tris | Potential Saved | Sample Draws |");
    println!("|---|---|---|---|---|---|");
    for c in &analysis.instancing.candidates {
        let draws_sample = c.draw_indices.iter().take(4).map(|i| format!("#{}", i)).collect::<Vec<_>>().join(", ");
        println!(
            "| {:.1} KB | {} | **{}** | {} | **-{} calls** | {} |",
            c.position_buffer_bytes as f64 / 1024.0,
            report::format_num(c.triangles_per_instance),
            c.draw_count,
            report::format_num(c.total_triangles),
            c.potential_saved_draws,
            draws_sample
        );
    }
}
