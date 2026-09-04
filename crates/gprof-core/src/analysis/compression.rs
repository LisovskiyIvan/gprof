use crate::model::TextureItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpotTextureItem {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub memory_bytes: usize,
    pub draw_count: usize,
    pub is_float: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionAnalysis {
    pub total_textures: usize,
    pub uncompressed_color_count: usize,
    pub uncompressed_color_bytes: usize,
    pub estimated_ktx2_bc7_bytes: usize,
    pub estimated_ktx2_savings_bytes: usize,
    pub float_vat_bytes: usize,
    pub estimated_half_float_savings_bytes: usize,
    pub npot_count: usize,
    pub npot_textures: Vec<NpotTextureItem>,
    pub warnings: Vec<String>,
}

pub fn analyze_compression(textures: &[TextureItem]) -> CompressionAnalysis {
    let mut uncompressed_color_count = 0;
    let mut uncompressed_color_bytes = 0;
    let mut float_vat_bytes = 0;
    let mut npot_textures = Vec::new();

    for t in textures {
        let is_float = t.format_name.contains("FLOAT")
            || t.internal_format_name.contains("FLOAT")
            || t.internal_format_name.contains("32F")
            || t.type_name.contains("FLOAT");

        if !t.is_pot {
            npot_textures.push(NpotTextureItem {
                id: t.id,
                width: t.width,
                height: t.height,
                memory_bytes: t.length,
                draw_count: t.draw_count,
                is_float,
            });
        }

        if is_float {
            float_vat_bytes += t.length;
        } else if !t.is_compressed {
            uncompressed_color_count += 1;
            uncompressed_color_bytes += t.length;
        }
    }

    // BC7 / ASTC 4x4 compresses RGBA8 (4 bytes/px) to 1 byte/px (4x reduction)
    let estimated_ktx2_bc7_bytes = uncompressed_color_bytes / 4;
    let estimated_ktx2_savings_bytes = uncompressed_color_bytes.saturating_sub(estimated_ktx2_bc7_bytes);

    // Half-float RGBA16F (8 bytes/px) saves 50% over RGBA32F (16 bytes/px)
    let estimated_half_float_savings_bytes = float_vat_bytes / 2;

    let mut warnings = Vec::new();
    if uncompressed_color_bytes > 50 * 1024 * 1024 {
        let mb = estimated_ktx2_savings_bytes as f64 / (1024.0 * 1024.0);
        warnings.push(format!(
            "{} uncompressed color textures take {:.1} MB VRAM. Converting to KTX2 (Basis Universal / BC7 / ASTC) can save up to {:.1} MB VRAM (-75%).",
            uncompressed_color_count,
            uncompressed_color_bytes as f64 / (1024.0 * 1024.0),
            mb
        ));
    }

    if float_vat_bytes > 50 * 1024 * 1024 {
        let mb = estimated_half_float_savings_bytes as f64 / (1024.0 * 1024.0);
        warnings.push(format!(
            "Float/VAT textures take {:.1} MB VRAM. If precision allows, switching to HALF_FLOAT (RGBA16F) saves {:.1} MB VRAM (-50%).",
            float_vat_bytes as f64 / (1024.0 * 1024.0),
            mb
        ));
    }

    let non_float_npot = npot_textures.iter().filter(|t| !t.is_float).count();
    if non_float_npot > 0 {
        warnings.push(format!(
            "{} non-float color textures have Non-Power-of-Two (NPOT) dimensions. NPOT textures cannot generate mipmaps or repeat in standard WebGL1 and may slow down texture sampling.",
            non_float_npot
        ));
    }

    CompressionAnalysis {
        total_textures: textures.len(),
        uncompressed_color_count,
        uncompressed_color_bytes,
        estimated_ktx2_bc7_bytes,
        estimated_ktx2_savings_bytes,
        float_vat_bytes,
        estimated_half_float_savings_bytes,
        npot_count: npot_textures.len(),
        npot_textures,
        warnings,
    }
}
