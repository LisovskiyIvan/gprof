pub mod analysis;
pub mod gl_enum;
pub mod model;
pub mod parser;
pub mod report;

pub use analysis::*;
pub use model::*;
pub use parser::{parse_file, parse_slice};
pub use report::*;

pub fn version() -> &'static str {
    "0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_capture_end_to_end() {
        let path = "../../fixtures/capture 21_30_18.json";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let t0 = Instant::now();
        let capture = parse_file(path).expect("Failed to parse fixture");
        let parse_time = t0.elapsed();
        println!("Parse time: {:?}", parse_time);

        let t1 = Instant::now();
        let analysis = analyze(&capture, 20);
        let analyze_time = t1.elapsed();
        println!("Analyze time: {:?}", analyze_time);

        assert_eq!(capture.summary.draw_calls, 262);
        assert_eq!(capture.summary.total_commands, 2092);
        assert_eq!(capture.summary.dynamic_uploads_count, 5);
        assert_eq!(capture.summary.particle_systems_count, 19);
        assert_eq!(capture.summary.total_particles, 103000);
        assert_eq!(analysis.glow.black_occluder_draws, 88);
        assert_eq!(analysis.glow.active_glowing_draws, 14);

        let report = format_full_report(&capture, &analysis);
        assert!(report.contains("Executive Summary"));
        assert!(report.contains("GlowLayer"));
        println!("Report length: {} chars", report.len());
    }
}
