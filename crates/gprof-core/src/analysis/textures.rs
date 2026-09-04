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
}

pub fn analyze_textures(textures: &[TextureItem]) -> TextureAnalysisResult {
    let total_count = textures.len();
    let total_bytes = textures.iter().map(|t| t.length).sum();

    let mut b_8k = Vec::new();
    let mut b_4k = Vec::new();
    let mut b_2k = Vec::new();
    let mut b_1k = Vec::new();
    let mut b_float = Vec::new();
    let mut b_small = Vec::new();

    let mut warnings = Vec::new();
    let mut single_draw_large = Vec::new();
    let mut emissive_large = Vec::new();

    for t in textures {
        let is_float = t.r#type == 5126 || t.internal_format == 34836 || t.internal_format == 34842;
        let max_dim = t.width.max(t.height);

        if is_float {
            b_float.push(t);
        } else if max_dim >= 8192 {
            b_8k.push(t);
        } else if max_dim >= 4096 {
            b_4k.push(t);
        } else if max_dim >= 2048 {
            b_2k.push(t);
        } else if max_dim >= 1024 {
            b_1k.push(t);
        } else {
            b_small.push(t);
        }

        // Warnings
        if max_dim >= 8192 && !is_float {
            warnings.push(TextureWarning {
                texture_id: t.id,
                width: t.width,
                height: t.height,
                length: t.length,
                kind: "OVERSIZED_8K".to_string(),
                message: format!(
                    "8K texture ({}x{}) takes {:.1} MB uncompressed VRAM. Consider downscaling to 2K/4K.",
                    t.width,
                    t.height,
                    t.length as f64 / (1024.0 * 1024.0)
                ),
            });
        }

        if max_dim >= 2048 && !t.is_compressed && !is_float {
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
                        "2K Emissive map ({}x{}, {:.1} MB). Emissive maps are mostly dark; downscaling saves VRAM.",
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
                        "2K texture ({}x{}, {:.1} MB) bound in only {} draw call.",
                        t.width,
                        t.height,
                        t.length as f64 / (1024.0 * 1024.0),
                        t.draw_count
                    ),
                });
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
        make_bucket("Skybox / 8K+ (>= 8192)", b_8k),
        make_bucket("4K Textures (4096..8192)", b_4k),
        make_bucket("2K Textures (2048..4096)", b_2k),
        make_bucket("1K Textures (1024..2048)", b_1k),
        make_bucket("Float / VAT Data", b_float),
        make_bucket("Small Textures (< 1024)", b_small),
    ];

    TextureAnalysisResult {
        total_count,
        total_bytes,
        buckets: buckets.into_iter().filter(|b| b.count > 0).collect(),
        warnings,
        single_draw_large,
        emissive_large,
    }
}
