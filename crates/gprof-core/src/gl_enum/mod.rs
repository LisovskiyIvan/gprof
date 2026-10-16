use crate::model::{CompressionStatus, TextureEstimationMethod};

mod names;

pub use names::*;

/// Returns (block_width, block_height, bytes_per_block) if the format is block-compressed.
pub fn compressed_block_info(format: u32) -> Option<(u32, u32, usize)> {
    match format {
        // ETC1 (4x4, 8 bytes = 0.5 B/px)
        36196 => Some((4, 4, 8)),

        // ETC2 / EAC (4x4)
        // 8 bytes per 4x4 block: RGB8, SRGB8, RGB8_PUNCHTHROUGH, SRGB8_PUNCHTHROUGH, R11_EAC, SIGNED_R11_EAC
        37488 | 37489 | 37492 | 37493 | 37494 | 37495 => Some((4, 4, 8)),
        // 16 bytes per 4x4 block: RGBA8_EAC, SRGB8_ALPHA8_EAC, RG11_EAC, SIGNED_RG11_EAC
        37490 | 37491 | 37496 | 37497 => Some((4, 4, 16)),

        // S3TC / DXT
        // DXT1: 8 bytes per 4x4 block
        33776 | 33777 | 35916 | 35917 => Some((4, 4, 8)),
        // DXT3 / DXT5: 16 bytes per 4x4 block
        33778 | 33779 | 35918 | 35919 => Some((4, 4, 16)),

        // BPTC (BC7, BC6H): 16 bytes per 4x4 block
        36492..=36495 => Some((4, 4, 16)),

        // RGTC (BC4: 8 bytes, BC5: 16 bytes per 4x4 block)
        36283 | 36284 => Some((4, 4, 8)),
        36285 | 36286 => Some((4, 4, 16)),

        // ASTC 2D formats (all ASTC blocks are 16 bytes)
        37808 | 37840 => Some((4, 4, 16)),
        37809 | 37841 => Some((5, 4, 16)),
        37810 | 37842 => Some((5, 5, 16)),
        37811 | 37843 => Some((6, 5, 16)),
        37812 | 37844 => Some((6, 6, 16)),
        37813 | 37845 => Some((8, 5, 16)),
        37814 | 37846 => Some((8, 6, 16)),
        37815 | 37847 => Some((8, 8, 16)),
        37816 | 37848 => Some((10, 5, 16)),
        37817 | 37849 => Some((10, 6, 16)),
        37818 | 37850 => Some((10, 8, 16)),
        37819 | 37851 => Some((10, 10, 16)),
        37820 | 37852 => Some((12, 10, 16)),
        37821 | 37853 => Some((12, 12, 16)),

        _ => None,
    }
}

pub fn is_compressed_format(format: u32) -> bool {
    compressed_block_info(format).is_some()
}

/// Returns bytes per pixel for uncompressed internal formats or format+type pairs.
pub fn uncompressed_bytes_per_pixel(internal_format: u32, format: u32, type_val: u32) -> Option<usize> {
    // 1. Check known sized internal formats first
    match internal_format {
        // 16 bytes per pixel
        34836 | 36202 | 36222 => return Some(16), // RGBA32F, RGBA32UI, RGBA32I
        // 12 bytes per pixel
        34837 | 36209 | 36227 => return Some(12), // RGB32F, RGB32UI, RGB32I
        // 8 bytes per pixel
        33328 | 34842 | 36208 | 36226 | 36013 => return Some(8), // RG32F, RGBA16F, RGBA16UI, RGBA16I, DEPTH32F_STENCIL8
        // 6 bytes per pixel
        34843 | 36215 | 36233 => return Some(6), // RGB16F, RGB16UI, RGB16I
        // 4 bytes per pixel
        32856 | 35907 | 33327 | 33326 | 35056 | 36012 | 36220 | 36238 | 32857 | 36975 | 35898 | 35901 => {
            return Some(4); // RGBA8, SRGB8_ALPHA8, RG16F, R32F, DEPTH24_STENCIL8, DEPTH_COMPONENT32F, RGBA8UI, RGBA8I, RGB10_A2, RGB10_A2UI, R11F_G11F_B10F, RGB9_E5
        }
        // 3 bytes per pixel
        32849 | 35905 | 36221 | 36239 => return Some(3), // RGB8, SRGB8, RGB8UI, RGB8I
        // 2 bytes per pixel
        33323 | 33325 | 33189 | 36194 | 32854 | 32855 | 33337 | 33338 => return Some(2), // RG8, R16F, DEPTH_COMPONENT16, RGB565, RGBA4, RGB5_A1, RG16I, RG16UI
        // 1 byte per pixel
        33321 | 33330 | 33329 | 36168 => return Some(1), // R8, R8UI, R8I, STENCIL_INDEX8
        _ => {}
    }

    // 2. Try base format + type combination
    let components = match format {
        6408 | 36249 => 4,                      // RGBA, RGBA_INTEGER
        6407 | 36248 => 3,                      // RGB, RGB_INTEGER
        33319 | 33320 => 2,                     // RG, RG_INTEGER
        6406 | 6409 | 6402 | 6403 | 36244 => 1, // ALPHA, LUMINANCE, DEPTH_COMPONENT, RED, RED_INTEGER
        6410 | 34041 => 2,                      // LUMINANCE_ALPHA, DEPTH_STENCIL
        _ => return None,
    };

    let bytes_per_component = match type_val {
        5126 => 4,                                               // FLOAT
        5131 | 36193 => 2,                                       // HALF_FLOAT, HALF_FLOAT_OES
        5121 | 5120 => 1,                                        // UNSIGNED_BYTE, BYTE
        5123 | 5122 => 2,                                        // UNSIGNED_SHORT, SHORT
        5125 | 5124 => 4,                                        // UNSIGNED_INT, INT
        32819 | 32820 | 33635 => return Some(2),                 // Packed 16-bit
        33640 | 34042 | 35899 | 35902 | 36269 => return Some(4), // Packed 32-bit
        _ => return None,
    };

    Some(components * bytes_per_component)
}

pub fn is_float_or_half_float(internal_format: u32, _format: u32, type_val: u32) -> bool {
    // Types
    if type_val == 5126 || type_val == 5131 || type_val == 36193 {
        return true;
    }
    // Internal formats
    matches!(internal_format, 34836 | 34837 | 34842 | 34843 | 33325 | 33326 | 33327 | 33328 | 35898 | 35901 | 36012 | 36013)
}

/// Calculate VRAM usage for a texture, handling block compression, mipmaps, faces, and estimation method.
/// Returns (total_vram_bytes, base_vram_bytes, mip_levels, CompressionStatus, TextureEstimationMethod).
pub fn calculate_texture_memory(
    width: u32,
    height: u32,
    internal_format: u32,
    format: u32,
    type_val: u32,
    target: &str,
    raw_length: usize,
) -> (usize, usize, usize, CompressionStatus, TextureEstimationMethod) {
    if width == 0 || height == 0 {
        if raw_length > 0 {
            return (raw_length, raw_length, 1, CompressionStatus::Unknown, TextureEstimationMethod::EstimatedFallback);
        }
        return (0, 0, 0, CompressionStatus::Unknown, TextureEstimationMethod::Unknown);
    }

    let faces = if target == "TEXTURE_CUBE_MAP" { 6 } else { 1 };
    let is_pot = width.is_power_of_two() && height.is_power_of_two();

    // 1. Compressed texture format
    let comp_fmt = if is_compressed_format(internal_format) {
        Some(internal_format)
    } else if is_compressed_format(format) {
        Some(format)
    } else {
        None
    };

    if let Some(cfmt) = comp_fmt {
        if let Some((bw, bh, bpb)) = compressed_block_info(cfmt) {
            let bx0 = width.div_ceil(bw);
            let by0 = height.div_ceil(bh);
            let base_bytes = (bx0 as usize) * (by0 as usize) * bpb * faces;

            // Calculate with full mip pyramid if POT or multiple levels
            let mut total_bytes = 0;
            let mut w = width;
            let mut h = height;
            let mut levels = 0;
            loop {
                let bx = w.div_ceil(bw);
                let by = h.div_ceil(bh);
                total_bytes += (bx as usize) * (by as usize) * bpb * faces;
                levels += 1;
                if w == 1 && h == 1 {
                    break;
                }
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }

            // For POT compressed textures, full mipmaps are standard.
            let chosen_bytes = if is_pot && (width > 4 || height > 4) { total_bytes } else { base_bytes };

            return (chosen_bytes, base_bytes, levels, CompressionStatus::Compressed, TextureEstimationMethod::FormatCalculation);
        }
    }

    // 2. Uncompressed format with known bytes per pixel
    if let Some(bpp) = uncompressed_bytes_per_pixel(internal_format, format, type_val) {
        let base_bytes = (width as usize) * (height as usize) * bpp * faces;
        let is_float = is_float_or_half_float(internal_format, format, type_val);

        // Compute mip pyramid
        let mut total_bytes = 0;
        let mut w = width;
        let mut h = height;
        let mut levels = 0;
        loop {
            total_bytes += (w as usize) * (h as usize) * bpp * faces;
            levels += 1;
            if w == 1 && h == 1 {
                break;
            }
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }

        // Check if raw_length matches base_bytes or total_bytes exactly (Known Storage)
        if raw_length > 0 && (raw_length == base_bytes || raw_length == total_bytes) {
            return (
                raw_length,
                base_bytes,
                if raw_length == total_bytes { levels } else { 1 },
                CompressionStatus::Uncompressed,
                TextureEstimationMethod::StorageSize,
            );
        }

        // If POT and not float animation data, assume standard mipmaps
        let chosen_bytes = if is_pot && !is_float && (width > 1 || height > 1) { total_bytes } else { base_bytes };

        return (
            chosen_bytes,
            base_bytes,
            if chosen_bytes == total_bytes { levels } else { 1 },
            CompressionStatus::Uncompressed,
            TextureEstimationMethod::FormatCalculation,
        );
    }

    // 3. Unknown format: if raw_length is present and non-zero
    if raw_length > 0 {
        return (raw_length, raw_length, 1, CompressionStatus::Unknown, TextureEstimationMethod::EstimatedFallback);
    }

    // 4. Fallback guess (4 B/px uncompressed assumption)
    let fallback_base = (width as usize) * (height as usize) * 4 * faces;
    (fallback_base, fallback_base, 1, CompressionStatus::Unknown, TextureEstimationMethod::EstimatedFallback)
}
