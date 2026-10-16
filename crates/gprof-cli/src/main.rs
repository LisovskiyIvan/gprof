mod print;

use print::*;

use clap::{Parser, Subcommand};
use gprof_core::{analysis, parse_file, report, ComprehensiveAnalysis, FrameCaptureSummary};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Serialize)]
struct JsonReport<'a> {
    parse_time_ms: f64,
    analyze_time_ms: f64,
    summary: &'a FrameCaptureSummary,
    analysis: &'a ComprehensiveAnalysis,
}

#[derive(Parser, Debug)]
#[command(
    name = "gprof",
    version = "0.1.0",
    about = "Ultra-fast SpectorJS WebGL / WebGPU capture analyzer & profiler",
    long_about = "gprof is a native Rust profiler for SpectorJS JSON frame captures.\nIt rapidly surfaces actionable insights: texture VRAM hogging, GlowLayer occluder waste, shader permutations, and GPU particle bottlenecks.",
    subcommand_precedence_over_arg = true
)]
struct Cli {
    /// Path to SpectorJS JSON capture file (.json or .json.gz)
    #[arg(value_name = "CAPTURE_JSON")]
    capture: Option<PathBuf>,

    /// Optional second capture file to compare against (A/B testing)
    #[arg(short = 'c', long, value_name = "AFTER_JSON")]
    compare: Option<PathBuf>,

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

    /// Show only instancing & batching candidates
    #[arg(short = 'i', long)]
    instancing: bool,

    /// Show only WebGL state thrashing & redundant calls audit
    #[arg(long)]
    states: bool,

    /// Show only geometry buffers, dynamic uploads (bufferSubData), and 32-bit index buffers
    #[arg(long)]
    buffers: bool,

    /// Show only Babylon.js performance linter rules & TypeScript code fixes
    #[arg(long)]
    lint: bool,

    /// Show only material & shader sorting efficiency and ping-pong switches
    #[arg(long)]
    sorting: bool,

    /// Show only texture compression (KTX2/ASTC) and NPOT analysis
    #[arg(long)]
    compression: bool,

    /// Show only render targets (FBO) VRAM footprint
    #[arg(long)]
    fbos: bool,

    /// Show only uniforms overhead & redundancy audit
    #[arg(long)]
    uniforms: bool,

    /// Show only screen-space overdraw & fillrate analysis
    #[arg(long)]
    overdraw: bool,

    /// Show only reconstructed scene tree and mesh hierarchy
    #[arg(long, alias = "scene")]
    tree: bool,

    /// Generate standalone interactive HTML dashboard (e.g. --html report.html)
    #[arg(long, value_name = "HTML_FILE")]
    html: Option<PathBuf>,

    /// Filter items by substring (textures, shaders, or draw calls)
    #[arg(long, global = true, value_name = "PATTERN")]
    filter: Option<String>,

    /// Filter draw calls by target FBO name / ID
    #[arg(long, global = true, value_name = "FBO")]
    fbo: Option<String>,

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
    Analyze { capture: PathBuf },
    /// Compare two captures (before vs after)
    Compare { before: PathBuf, after: PathBuf },
    /// Detailed texture breakdown & VRAM usage
    Textures { capture: PathBuf },
    /// GlowLayer & occluder waste audit
    Glow { capture: PathBuf },
    /// Heaviest draw calls by triangle count
    Draws { capture: PathBuf },
    /// Render passes sequence grouped by FBO
    Passes { capture: PathBuf },
    /// Shaders & PBR variant permutations
    Shaders { capture: PathBuf },
    /// GPU particle systems & transform feedback
    Particles { capture: PathBuf },
    /// Dynamic per-frame texture uploads (texImage2D)
    Uploads { capture: PathBuf },
    /// Detect duplicate geometry candidates for Thin/Hardware Instancing
    Instancing { capture: PathBuf },
    /// WebGL state thrashing & redundant calls audit
    States { capture: PathBuf },
    /// Geometry buffers, dynamic uploads (bufferSubData), and 32-bit index buffers
    Buffers { capture: PathBuf },
    /// Babylon.js performance linter rules & TypeScript code fixes
    Lint { capture: PathBuf },
    /// Material & shader sorting efficiency & ping-pong switches
    Sorting { capture: PathBuf },
    /// Texture compression (KTX2/ASTC) and NPOT analysis
    Compression { capture: PathBuf },
    /// Render targets (FBO) VRAM footprint
    Fbos { capture: PathBuf },
    /// Uniforms overhead & redundancy audit
    Uniforms { capture: PathBuf },
    /// Screen-space overdraw & fillrate pressure analysis
    Overdraw { capture: PathBuf },
    /// Reconstruct logical scene tree and mesh hierarchy
    #[command(alias = "scene")]
    Tree { capture: PathBuf },
    /// Generate standalone interactive HTML dashboard
    Html {
        capture: PathBuf,
        /// Destination HTML file path (default: report.html)
        #[arg(short, long, default_value = "report.html")]
        output: PathBuf,
    },
    /// Export full report as Markdown
    Export {
        capture: PathBuf,
        /// Optional destination file (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

impl Commands {
    fn capture(&self) -> &PathBuf {
        match self {
            Commands::Analyze { capture }
            | Commands::Textures { capture }
            | Commands::Glow { capture }
            | Commands::Draws { capture }
            | Commands::Passes { capture }
            | Commands::Shaders { capture }
            | Commands::Particles { capture }
            | Commands::Uploads { capture }
            | Commands::Instancing { capture }
            | Commands::States { capture }
            | Commands::Buffers { capture }
            | Commands::Lint { capture }
            | Commands::Sorting { capture }
            | Commands::Compression { capture }
            | Commands::Fbos { capture }
            | Commands::Uniforms { capture }
            | Commands::Overdraw { capture }
            | Commands::Tree { capture }
            | Commands::Html { capture, .. }
            | Commands::Export { capture, .. } => capture,
            Commands::Compare { before, .. } => before,
        }
    }

    fn subcommand(&self) -> Option<&'static str> {
        match self {
            Commands::Analyze { .. } => None,
            Commands::Textures { .. } => Some("textures"),
            Commands::Glow { .. } => Some("glow"),
            Commands::Draws { .. } => Some("draws"),
            Commands::Passes { .. } => Some("passes"),
            Commands::Shaders { .. } => Some("shaders"),
            Commands::Particles { .. } => Some("particles"),
            Commands::Uploads { .. } => Some("uploads"),
            Commands::Instancing { .. } => Some("instancing"),
            Commands::States { .. } => Some("states"),
            Commands::Buffers { .. } => Some("buffers"),
            Commands::Lint { .. } => Some("lint"),
            Commands::Sorting { .. } => Some("sorting"),
            Commands::Compression { .. } => Some("compression"),
            Commands::Fbos { .. } => Some("fbos"),
            Commands::Uniforms { .. } => Some("uniforms"),
            Commands::Overdraw { .. } => Some("overdraw"),
            Commands::Tree { .. } => Some("tree"),
            Commands::Html { .. } => Some("html"),
            Commands::Export { .. } => Some("export"),
            Commands::Compare { .. } => None,
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // Check compare mode first
    if let Some(Commands::Compare { before, after }) = &cli.command {
        run_compare(before, after, &cli);
        return;
    }
    if let (Some(before), Some(after)) = (&cli.capture, &cli.compare) {
        run_compare(before, after, &cli);
        return;
    }

    let (path, sub_cmd) = if let Some(cmd) = &cli.command {
        (cmd.capture().clone(), cmd.subcommand())
    } else if let Some(capture) = &cli.capture {
        (capture.clone(), None)
    } else {
        eprintln!("Error: No capture file specified.\nUsage: gprof <CAPTURE_JSON> [OPTIONS]\nRun 'gprof --help' for details.");
        process::exit(1);
    };

    if !path.exists() {
        eprintln!("Error: File not found: {}", path.display());
        process::exit(1);
    }

    let t0 = std::time::Instant::now();
    let mut capture = match parse_file(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing SpectorJS capture: {}", e);
            process::exit(1);
        }
    };
    let parse_time = t0.elapsed();

    // Apply filters if requested
    if let Some(pat) = &cli.filter {
        let pat_lower = pat.to_lowercase();
        capture
            .textures
            .retain(|t| t.format_name.to_lowercase().contains(&pat_lower) || t.uniform_names.iter().any(|u| u.to_lowercase().contains(&pat_lower)));
        capture
            .draws
            .retain(|d| d.shader_name.to_lowercase().contains(&pat_lower) || d.text.to_lowercase().contains(&pat_lower));
        capture.programs.retain(|p| p.shader_name.to_lowercase().contains(&pat_lower));
    }

    if let Some(fbo_filter) = &cli.fbo {
        let fbo_lower = fbo_filter.to_lowercase();
        capture.draws.retain(|d| d.fbo_name.to_lowercase().contains(&fbo_lower));
        capture.passes.retain(|p| p.fbo_name.to_lowercase().contains(&fbo_lower));
    }

    let t1 = std::time::Instant::now();
    let analysis = analysis::analyze(&capture, cli.top);
    let analyze_time = t1.elapsed();

    if cli.json {
        let json = JsonReport {
            parse_time_ms: parse_time.as_secs_f64() * 1000.0,
            analyze_time_ms: analyze_time.as_secs_f64() * 1000.0,
            summary: &capture.summary,
            analysis: &analysis,
        };
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return;
    }

    if let Some(html_path) = &cli.html {
        let html_content = gprof_core::generate_html_report(&capture, &analysis);
        if let Err(e) = fs::write(html_path, html_content) {
            eprintln!("Error writing HTML report to {}: {}", html_path.display(), e);
            process::exit(1);
        }
        println!("✨ Interactive HTML dashboard generated at: {}", html_path.display());
        return;
    }

    let is_textures = cli.textures || sub_cmd == Some("textures");
    let is_glow = cli.glow || sub_cmd == Some("glow");
    let is_draws = cli.draws || sub_cmd == Some("draws");
    let is_passes = cli.passes || sub_cmd == Some("passes");
    let is_shaders = cli.shaders || sub_cmd == Some("shaders");
    let is_particles = cli.particles || sub_cmd == Some("particles");
    let is_uploads = cli.uploads || sub_cmd == Some("uploads");
    let is_instancing = cli.instancing || sub_cmd == Some("instancing");
    let is_states = cli.states || sub_cmd == Some("states");
    let is_buffers = cli.buffers || sub_cmd == Some("buffers");
    let is_lint = cli.lint || sub_cmd == Some("lint");
    let is_sorting = cli.sorting || sub_cmd == Some("sorting");
    let is_compression = cli.compression || sub_cmd == Some("compression");
    let is_fbos = cli.fbos || sub_cmd == Some("fbos");
    let is_uniforms = cli.uniforms || sub_cmd == Some("uniforms");
    let is_overdraw = cli.overdraw || sub_cmd == Some("overdraw");
    let is_tree = cli.tree || sub_cmd == Some("tree") || sub_cmd == Some("scene");
    let is_html = sub_cmd == Some("html");
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
    } else if is_instancing {
        print_instancing(&analysis);
    } else if is_states {
        print!("{}", report::format_states_table(&analysis.states));
    } else if is_buffers {
        print!("{}", report::format_buffers_table(&analysis.buffers));
    } else if is_lint {
        print!("{}", analysis::format_lint_report(&analysis.lint));
    } else if is_sorting {
        print!("{}", report::format_sorting_table(&analysis.sorting));
    } else if is_compression {
        print!("{}", report::format_compression_table(&analysis.compression));
    } else if is_fbos {
        print!("{}", report::format_fbo_table(&analysis.fbos));
    } else if is_uniforms {
        print!("{}", report::format_uniforms_table(&analysis.uniforms));
    } else if is_overdraw {
        print!("{}", report::format_overdraw_table(&analysis.overdraw));
    } else if is_tree {
        print!("{}", report::format_scene_tree_table(&analysis.tree, cli.top));
    } else if is_html {
        let out_path = match &cli.command {
            Some(Commands::Html { output, .. }) => output.clone(),
            _ => PathBuf::from("report.html"),
        };
        let html_content = gprof_core::generate_html_report(&capture, &analysis);
        if let Err(e) = fs::write(&out_path, html_content) {
            eprintln!("Error writing HTML report to {}: {}", out_path.display(), e);
            process::exit(1);
        }
        println!("✨ Interactive HTML dashboard generated at: {}", out_path.display());
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

fn run_compare(before_path: &Path, after_path: &Path, cli: &Cli) {
    if !before_path.exists() {
        eprintln!("Error: Before file not found: {}", before_path.display());
        process::exit(1);
    }
    if !after_path.exists() {
        eprintln!("Error: After file not found: {}", after_path.display());
        process::exit(1);
    }

    let before = parse_file(before_path).expect("Failed to parse before capture");
    let after = parse_file(after_path).expect("Failed to parse after capture");

    let comp = analysis::compare_captures(&before, &after);

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&comp).unwrap());
        return;
    }

    let md = analysis::format_compare_markdown(&before, &after, &comp);
    print!("{}", md);
}

fn write_export_file(path: &Path, content: &str) {
    match fs::write(path, content) {
        Ok(_) => eprintln!("Saved Markdown report to {}", path.display()),
        Err(e) => eprintln!("Failed to write report to {}: {}", path.display(), e),
    }
}

