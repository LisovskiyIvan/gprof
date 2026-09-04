use clap::{Parser, Subcommand};
use gprof_core::{analysis, parse_file, report, ComprehensiveAnalysis, ParsedCapture};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser, Debug)]
#[command(
    name = "gprof",
    version = "0.1.0",
    about = "Ultra-fast SpectorJS WebGL / WebGPU capture analyzer & profiler",
    long_about = "gprof is a native Rust profiler for SpectorJS JSON frame captures.\nIt rapidly surfaces actionable insights: texture VRAM hogging, GlowLayer occluder waste, shader permutations, and GPU particle bottlenecks."
)]
struct Cli {
    /// Path to SpectorJS JSON capture file (e.g. capture.json)
    #[arg(value_name = "CAPTURE_JSON")]
    capture: Option<PathBuf>,

    /// Subcommand for targeted inspection
    #[command(subcommand)]
    command: Option<Commands>,

    /// Show only texture memory breakdown
    #[arg(short = 't', long)]
    textures: bool,

    /// Show only GlowLayer occluder audit
    #[arg(short = 'g', long)]
    glow: bool,

    /// Show only heaviest draw calls
    #[arg(short = 'd', long)]
    draws: bool,

    /// Show only render passes sequence
    #[arg(short = 'p', long)]
    passes: bool,

    /// Show only shaders & PBR variants analysis
    #[arg(short = 's', long)]
    shaders: bool,

    /// Show only GPU particles / transform feedback
    #[arg(long)]
    particles: bool,

    /// Show only dynamic per-frame texture uploads (texImage2D)
    #[arg(long)]
    uploads: bool,

    /// Export analysis as Markdown to file or stdout (default stdout, e.g. --export=report.md)
    #[arg(short = 'e', long, global = true, num_args = 0..=1, default_missing_value = "-")]
    export: Option<String>,

    /// Print machine-readable JSON output
    #[arg(long, global = true)]
    json: bool,

    /// Max rows to show in tables (default: 20)
    #[arg(long, global = true, default_value_t = 20)]
    top: usize,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Full frame analysis (default)
    Analyze {
        capture: PathBuf,
    },
    /// Detailed texture breakdown & VRAM usage
    Textures {
        capture: PathBuf,
    },
    /// GlowLayer & occluder waste audit
    Glow {
        capture: PathBuf,
    },
    /// Heaviest draw calls by triangle count
    Draws {
        capture: PathBuf,
    },
    /// Render passes sequence grouped by FBO
    Passes {
        capture: PathBuf,
    },
    /// Shaders & PBR variant permutations
    Shaders {
        capture: PathBuf,
    },
    /// GPU particle systems & transform feedback
    Particles {
        capture: PathBuf,
    },
    /// Dynamic per-frame texture uploads (texImage2D)
    Uploads {
        capture: PathBuf,
    },
    /// Export full report as Markdown
    Export {
        capture: PathBuf,
        /// Optional destination file (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let (path, sub_cmd) = match (&cli.capture, &cli.command) {
        (Some(p), None) => (p.clone(), None),
        (None, Some(cmd)) => match cmd {
            Commands::Analyze { capture } => (capture.clone(), None),
            Commands::Textures { capture } => (capture.clone(), Some("textures")),
            Commands::Glow { capture } => (capture.clone(), Some("glow")),
            Commands::Draws { capture } => (capture.clone(), Some("draws")),
            Commands::Passes { capture } => (capture.clone(), Some("passes")),
            Commands::Shaders { capture } => (capture.clone(), Some("shaders")),
            Commands::Particles { capture } => (capture.clone(), Some("particles")),
            Commands::Uploads { capture } => (capture.clone(), Some("uploads")),
            Commands::Export { capture, .. } => (capture.clone(), Some("export")),
        },
        (Some(p), Some(cmd)) => match cmd {
            Commands::Analyze { .. } => (p.clone(), None),
            Commands::Textures { .. } => (p.clone(), Some("textures")),
            Commands::Glow { .. } => (p.clone(), Some("glow")),
            Commands::Draws { .. } => (p.clone(), Some("draws")),
            Commands::Passes { .. } => (p.clone(), Some("passes")),
            Commands::Shaders { .. } => (p.clone(), Some("shaders")),
            Commands::Particles { .. } => (p.clone(), Some("particles")),
            Commands::Uploads { .. } => (p.clone(), Some("uploads")),
            Commands::Export { .. } => (p.clone(), Some("export")),
        },
        (None, None) => {
            eprintln!("Error: No capture file specified.\nUsage: gprof <CAPTURE_JSON> [OPTIONS]\nRun 'gprof --help' for details.");
            process::exit(1);
        }
    };

    if !path.exists() {
        eprintln!("Error: File not found: {}", path.display());
        process::exit(1);
    }

    let t0 = std::time::Instant::now();
    let capture = match parse_file(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing SpectorJS capture: {}", e);
            process::exit(1);
        }
    };
    let parse_time = t0.elapsed();

    let t1 = std::time::Instant::now();
    let analysis = analysis::analyze(&capture, cli.top);
    let analyze_time = t1.elapsed();

    if cli.json {
        let json = serde_json::json!({
            "parse_time_ms": parse_time.as_secs_f64() * 1000.0,
            "analyze_time_ms": analyze_time.as_secs_f64() * 1000.0,
            "summary": capture.summary,
            "analysis": analysis,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return;
    }

    // Specific inspection mode requested
    let is_textures = cli.textures || sub_cmd == Some("textures");
    let is_glow = cli.glow || sub_cmd == Some("glow");
    let is_draws = cli.draws || sub_cmd == Some("draws");
    let is_passes = cli.passes || sub_cmd == Some("passes");
    let is_shaders = cli.shaders || sub_cmd == Some("shaders");
    let is_particles = cli.particles || sub_cmd == Some("particles");
    let is_uploads = cli.uploads || sub_cmd == Some("uploads");
    let is_export = cli.export.is_some() || sub_cmd == Some("export");

    if is_textures {
        print_textures(&capture, &analysis, cli.top);
    } else if is_glow {
        print_glow(&analysis);
    } else if is_draws {
        print_draws(&analysis, cli.top);
    } else if is_passes {
        print_passes(&analysis);
    } else if is_shaders {
        print_shaders(&analysis);
    } else if is_particles {
        print_particles(&analysis);
    } else if is_uploads {
        print_uploads(&analysis);
    } else if is_export {
        let report_md = report::format_full_report(&capture, &analysis);
        let export_target = cli.export.as_deref().unwrap_or("-");
        if export_target == "-" {
            if let Some(Commands::Export { output: Some(out_path), .. }) = &cli.command {
                write_export_file(out_path, &report_md);
            } else {
                print!("{}", report_md);
            }
        } else {
            write_export_file(Path::new(export_target), &report_md);
        }
    } else {
        // Full Executive Report
        let report_md = report::format_full_report(&capture, &analysis);
        print!("{}", report_md);
        eprintln!(
            "\n⚡ Analyzed in {:.1}ms (parse: {:.1}ms, analyze: {:.1}ms)",
            (parse_time + analyze_time).as_secs_f64() * 1000.0,
            parse_time.as_secs_f64() * 1000.0,
            analyze_time.as_secs_f64() * 1000.0
        );
    }
}

fn write_export_file(path: &Path, content: &str) {
    match fs::write(path, content) {
        Ok(_) => eprintln!("Saved Markdown report to {}", path.display()),
        Err(e) => eprintln!("Failed to write report to {}: {}", path.display(), e),
    }
}

fn print_textures(capture: &ParsedCapture, analysis: &ComprehensiveAnalysis, top: usize) {
    println!("# Textures Analysis (Total: {}, Memory: {})\n", capture.textures.len(), report::format_bytes(capture.summary.total_texture_bytes));

    println!("## Memory Categories\n");
    println!("| Category | Count | Total VRAM | Sample IDs |");
    println!("|---|---|---|---|");
    for b in &analysis.textures.buckets {
        let sample = b.sample_ids.iter().map(|id| format!("#{}", id)).collect::<Vec<_>>().join(", ");
        println!("| {} | {} | **{}** | {} |", b.name, b.count, report::format_bytes(b.total_bytes), sample);
    }
    println!();

    if !analysis.textures.warnings.is_empty() {
        println!("## Warnings & Optimization Opportunities\n");
        for w in &analysis.textures.warnings {
            println!("- ⚠️  [#{}] {}", w.texture_id, w.message);
        }
        println!();
    }

    println!("{}", report::format_textures_table(&capture.textures, top));
}

fn print_glow(analysis: &ComprehensiveAnalysis) {
    println!("# GlowLayer Occluder Audit\n");
    println!("- Total Glow Passes: **{}** draw calls, **{}** triangles", analysis.glow.total_draws, report::format_num(analysis.glow.total_triangles));
    println!("- 🔴 **Black Occluders:** **{}** draw calls, **{}** triangles (**{:.1}% wasted**)", analysis.glow.black_occluder_draws, report::format_num(analysis.glow.black_occluder_triangles), analysis.glow.wasted_triangle_percentage);
    println!("- 🟢 **Active Glowing:** **{}** draw calls, **{}** triangles\n", analysis.glow.active_glowing_draws, report::format_num(analysis.glow.active_glowing_triangles));

    println!("> [!IMPORTANT]\n> {}\n", analysis.glow.recommendation);

    if !analysis.glow.top_black_occluders.is_empty() {
        println!("## Heaviest Black Occluder Meshes:\n");
        println!("| Draw # | Command # | Triangles | Buffer (KB) | World Position (X, Y, Z) |");
        println!("|---|---|---|---|---|");
        for m in &analysis.glow.top_black_occluders {
            let pos = m.world_position.map(|p| format!("({:.1}, {:.1}, {:.1})", p[0], p[1], p[2])).unwrap_or_else(|| "—".to_string());
            println!("| #{} | #{} | **{}** | {:.1} KB | {} |", m.draw_index, m.command_id, report::format_num(m.triangles), m.vertex_buffer_bytes as f64 / 1024.0, pos);
        }
    }
}

fn print_draws(analysis: &ComprehensiveAnalysis, top: usize) {
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

fn print_passes(analysis: &ComprehensiveAnalysis) {
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

fn print_shaders(analysis: &ComprehensiveAnalysis) {
    println!("# Shaders & Program Permutations\n");
    println!("- Total WebGL Programs: **{}**", analysis.shaders.total_programs);
    println!("- `useProgram` calls: **{}** (redundant: {})", analysis.shaders.use_program_calls, analysis.shaders.redundant_use_program_calls);
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
        let diff_str = if v.unique_defines.is_empty() { "None (Baseline)".to_string() } else { v.unique_defines.join(", ") };
        println!("| #{} | {} | `{}` |", v.program_id, v.draw_count, diff_str);
    }
}

fn print_particles(analysis: &ComprehensiveAnalysis) {
    println!("# GPU Particles (Transform Feedback)\n");
    println!("- Active Systems: **{}**", analysis.particles.systems_count);
    println!("- Total Active Particles: **{}** points", report::format_num(analysis.particles.total_particles));
    println!("- Total Quad Triangles: **{}**\n", report::format_num(analysis.particles.total_quad_triangles));

    if let Some(w) = &analysis.particles.warning {
        println!("> [!WARNING]\n> {}\n", w);
    }

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

fn print_uploads(analysis: &ComprehensiveAnalysis) {
    println!("# Dynamic Texture Uploads\n");
    println!("- Total per-frame uploads: **{}**\n", analysis.uploads.total_uploads);

    if let Some(w) = &analysis.uploads.warning {
        println!("> [!WARNING]\n> {}\n", w);
    }

    println!("| # | Command | Target | Dimensions | Stack Trace |");
    println!("|---|---|---|---|---|");
    for (i, u) in analysis.uploads.uploads.iter().enumerate() {
        let st = u.stack_trace.iter().take(2).cloned().collect::<Vec<_>>().join(" → ");
        let st_str = if st.is_empty() { "—".to_string() } else { st };
        println!("| {} | {} | {} | {}x{} | `{}` |", i + 1, u.name, u.target, u.width, u.height, st_str);
    }
}
