use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureBucket {
    pub name: String,
    pub count: usize,
    pub total_bytes: usize,
    pub sample_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureWarning {
    pub texture_id: u32,
    pub width: u32,
    pub height: u32,
    pub length: usize,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureAnalysisResult {
    pub total_count: usize,
    pub total_bytes: usize,
    pub buckets: Vec<TextureBucket>,
    pub warnings: Vec<TextureWarning>,
    pub single_draw_large: Vec<u32>,
    pub emissive_large: Vec<u32>,
    pub color_bytes: usize,
    pub vat_bytes: usize,
    pub rt_bytes: usize,
    pub unknown_bytes: usize,
}

pub fn analyze_textures(textures: &[TextureItem]) -> TextureAnalysisResult {
    let total_count = textures.len();
    let total_bytes = textures.iter().map(|t| t.length).sum();

    let mut b_vat = Vec::new();
    let mut b_4k_plus = Vec::new();
    let mut b_2k = Vec::new();
    let mut b_1k = Vec::new();
    let mut b_small = Vec::new();
    let mut b_rt = Vec::new();
    let mut b_unknown = Vec::new();

    let mut warnings = Vec::new();
    let mut single_draw_large = Vec::new();
    let mut emissive_large = Vec::new();

    let mut color_bytes = 0;
    let mut vat_bytes = 0;
    let mut rt_bytes = 0;
    let mut unknown_bytes = 0;

    for t in textures {
        let max_dim = t.width.max(t.height);
        let min_dim = t.width.min(t.height);

        match t.category {
            TextureUsageCategory::GpuAnimationData => {
                vat_bytes += t.length;
                b_vat.push(t);
                // Do NOT generate downscale warnings for VAT textures:
                // downscaling vector animation textures corrupts vertex offsets.
                // Instead, check if switching to half-float (RGBA16F) is a candidate.
                if t.length >= 10 * 1024 * 1024 && (t.internal_format_name.contains("32F") || t.type_name.contains("FLOAT")) {
                    warnings.push(TextureWarning {
                        texture_id: t.id,
                        width: t.width,
                        height: t.height,
                        length: t.length,
                        kind: "VAT_PRECISION".to_string(),
                        message: format!(
                            "[Hypothesis / Candidate] High-precision 32-bit float VAT data texture ({}x{}, {:.1} MB). Test if 16-bit half-float (RGBA16F) provides sufficient vertex animation precision to cut VRAM in half.",
                            t.width,
                            t.height,
                            t.length as f64 / (1024.0 * 1024.0)
                        ),
                    });
                }
            }
            TextureUsageCategory::RenderTarget => {
                rt_bytes += t.length;
                b_rt.push(t);
            }
            TextureUsageCategory::Unknown => {
                unknown_bytes += t.length;
                b_unknown.push(t);
            }
            TextureUsageCategory::ColorMaterial => {
                color_bytes += t.length;
                if max_dim >= 4096 {
                    b_4k_plus.push(t);
                } else if max_dim >= 2048 {
                    b_2k.push(t);
                } else if max_dim >= 1024 {
                    b_1k.push(t);
                } else {
                    b_small.push(t);
                }

                // Warnings for ColorMaterial textures (excluding 1D LUTs / ramps)
                if max_dim >= 8192 && min_dim >= 512 {
                    let comp_status_str = match t.compression_status {
                        CompressionStatus::Compressed => "already compressed",
                        CompressionStatus::Uncompressed => "uncompressed",
                        CompressionStatus::Unknown => "unknown compression",
                    };
                    warnings.push(TextureWarning {
                        texture_id: t.id,
                        width: t.width,
                        height: t.height,
                        length: t.length,
                        kind: "OVERSIZED_8K".to_string(),
                        message: format!(
                            "[Calculated Estimate] Oversized 8K Color texture ({}x{}) takes {:.1} MB VRAM ({}). Consider downscaling to 2K/4K if visual difference is negligible.",
                            t.width,
                            t.height,
                            t.length as f64 / (1024.0 * 1024.0),
                            comp_status_str
                        ),
                    });
                }

                if max_dim >= 2048 && t.compression_status == CompressionStatus::Uncompressed {
                    let is_emissive = t.uniform_names.iter().any(|u| u.to_lowercase().contains("emissive"));
                    if is_emissive {
                        emissive_large.push(t.id);
                        warnings.push(TextureWarning {
                            texture_id: t.id,
                            width: t.width,
                            height: t.height,
                            length: t.length,
                            kind: "LARGE_EMISSIVE".to_string(),
                            message: format!(
                                "[Hypothesis / Candidate] 2K+ Emissive map ({}x{}, {:.1} MB). Emissive maps are mostly dark; downscaling or channel packing can save VRAM.",
                                t.width,
                                t.height,
                                t.length as f64 / (1024.0 * 1024.0)
                            ),
                        });
                    }

                    if t.draw_count <= 1 {
                        single_draw_large.push(t.id);
                        warnings.push(TextureWarning {
                            texture_id: t.id,
                            width: t.width,
                            height: t.height,
                            length: t.length,
                            kind: "SINGLE_DRAW_2K".to_string(),
                            message: format!(
                                "[Observed Fact] 2K+ Color texture ({}x{}, {:.1} MB) bound in only {} draw call. Verify if streaming or lower LOD is appropriate.",
                                t.width,
                                t.height,
                                t.length as f64 / (1024.0 * 1024.0),
                                t.draw_count
                            ),
                        });
                    }
                }
            }
        }
    }

    let make_bucket = |name: &str, items: Vec<&TextureItem>| -> TextureBucket {
        let count = items.len();
        let bytes = items.iter().map(|i| i.length).sum();
        let sample = items.iter().take(5).map(|i| i.id).collect();
        TextureBucket {
            name: name.to_string(),
            count,
            total_bytes: bytes,
            sample_ids: sample,
        }
    };

    let buckets = vec![
        make_bucket("GPU Animation / VAT Data", b_vat),
        make_bucket("Color / Material: 4K+ (>= 4096)", b_4k_plus),
        make_bucket("Color / Material: 2K (2048..4095)", b_2k),
        make_bucket("Color / Material: 1K (1024..2047)", b_1k),
        make_bucket("Color / Material: Small (< 1024)", b_small),
        make_bucket("Render Targets / Buffers", b_rt),
        make_bucket("Unknown / Other Formats", b_unknown),
    ];

    TextureAnalysisResult {
        total_count,
        total_bytes,
        buckets: buckets.into_iter().filter(|b| b.count > 0).collect(),
        warnings,
        single_draw_large,
        emissive_large,
        color_bytes,
        vat_bytes,
        rt_bytes,
        unknown_bytes,
    }
}

