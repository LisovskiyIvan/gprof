pub mod analysis;
pub mod gl_enum;
pub mod html;
pub mod model;
pub mod parser;
pub mod report;

pub use analysis::*;
pub use html::*;
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
    fn test_texture_memory_calculation_formats() {
        // 1. Uncompressed RGBA8 (4096 x 4096, 6408 = RGBA, 5121 = UNSIGNED_BYTE, target = "TEXTURE_2D")
        let (rgba8_total, rgba8_base, mips, comp, method) = gl_enum::calculate_texture_memory(4096, 4096, 6408, 6408, 5121, "TEXTURE_2D", 0);
        assert_eq!(comp, CompressionStatus::Uncompressed);
        assert_eq!(method, TextureEstimationMethod::FormatCalculation);
        assert_eq!(rgba8_base, 4096 * 4096 * 4); // 64 MB
        assert_eq!(mips, 13);
        assert!(rgba8_total > rgba8_base); // includes mipmaps ~85.3 MB

        // 2. Uncompressed RGBA32F (1024 x 1024, internalFormat 34836 = RGBA32F, 6408 = RGBA, 5126 = FLOAT)
        let (float_total, float_base, _, comp_f, method_f) = gl_enum::calculate_texture_memory(1024, 1024, 34836, 6408, 5126, "TEXTURE_2D", 0);
        assert_eq!(comp_f, CompressionStatus::Uncompressed);
        assert_eq!(method_f, TextureEstimationMethod::FormatCalculation);
        assert_eq!(float_base, 1024 * 1024 * 16); // 16 MB
        assert_eq!(float_total, 1024 * 1024 * 16); // float data texture does not generate mipmaps

        // 3. Compressed ETC2 RGB8 (internalFormat 37492 = COMPRESSED_RGB8_ETC2, 4096 x 4096)
        // Block size: 4x4, 8 bytes per block
        // Blocks: (4096/4) * (4096/4) = 1024 * 1024 = 1,048,576 blocks
        // Base bytes: 1,048,576 * 8 = 8,388,608 bytes (8.0 MB)
        let (etc2_total, etc2_base, mips_etc2, comp_etc2, method_etc2) = gl_enum::calculate_texture_memory(4096, 4096, 0, 37492, 0, "TEXTURE_2D", 0);
        assert_eq!(comp_etc2, CompressionStatus::Compressed);
        assert_eq!(method_etc2, TextureEstimationMethod::FormatCalculation);
        assert_eq!(etc2_base, 8 * 1024 * 1024); // 8 MB (NOT 64 MB!)
        assert_eq!(mips_etc2, 13);
        assert_eq!(etc2_total, 11_184_824); // ~10.67 MB with mip levels (min 1 block per mip)

        // 4. Compressed ASTC 4x4 (internalFormat 37808 = COMPRESSED_RGBA_ASTC_4x4_KHR, 4096 x 4096)
        // Block size: 4x4, 16 bytes per block -> Base bytes: 16 MB
        let (astc_total, astc_base, _, comp_astc, _) = gl_enum::calculate_texture_memory(4096, 4096, 0, 37808, 0, "TEXTURE_2D", 0);
        assert_eq!(comp_astc, CompressionStatus::Compressed);
        assert_eq!(astc_base, 16 * 1024 * 1024);
        assert!(astc_total > astc_base);

        // 5. Unknown format fallback
        let (unk_total, unk_base, _, comp_unk, method_unk) = gl_enum::calculate_texture_memory(512, 512, 0, 99999, 0, "TEXTURE_2D", 0);
        assert_eq!(comp_unk, CompressionStatus::Unknown);
        assert_eq!(method_unk, TextureEstimationMethod::EstimatedFallback);
        assert_eq!(unk_base, 512 * 512 * 4);
        assert!(unk_total >= unk_base);
    }

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

        // Verify textures #92, #94, #95, #96, #98, #99
        let t92 = capture.textures.iter().find(|t| t.id == 92).expect("Texture #92 should exist");
        assert_eq!(t92.internal_format, 37492);
        assert_eq!(t92.internal_format_name, "COMPRESSED_RGB8_ETC2");
        assert_eq!(t92.compression_status, CompressionStatus::Compressed);
        assert!(t92.is_compressed);
        assert_eq!(t92.base_bytes, 8 * 1024 * 1024); // 8 MB base, not 64 MB!
        assert!(t92.length < 12 * 1024 * 1024); // with mips ~10.7 MB, NOT 64 MB!

        // Verify VAT textures are classified as GpuAnimationData, not Skybox
        let vat_tex = capture.textures.iter().find(|t| t.height >= 8192);
        assert!(vat_tex.is_some());
        let vat = vat_tex.unwrap();
        assert_eq!(vat.category, TextureUsageCategory::GpuAnimationData);

        // Verify instancing analysis does not group across passes
        for candidate in &analysis.instancing.candidates {
            for &draw_idx in &candidate.draw_indices {
                assert_eq!(capture.draws[draw_idx].pass_index, candidate.pass_index);
            }
        }

        let report = format_full_report(&capture, &analysis);
        assert!(report.contains("Executive Summary"));
        assert!(report.contains("GlowLayer"));
        assert!(report.contains("[Observed Fact]"));
        assert!(report.contains("[Calculated Estimate]"));
        assert!(report.contains("[Hypothesis / Candidate]"));
        println!("Report length: {} chars", report.len());
    }
}
