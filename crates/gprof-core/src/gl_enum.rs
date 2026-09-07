//! WebGL constants, enum names, and memory calculation helpers.

use crate::model::{CompressionStatus, TextureEstimationMethod};

/// Returns human-readable name for GL formats and internal formats.
pub fn format_name(val: u32) -> &'static str {
    match val {
        // Base formats
        6406 => "ALPHA",
        6407 => "RGB",
        6408 => "RGBA",
        6409 => "LUMINANCE",
        6410 => "LUMINANCE_ALPHA",
        6402 => "DEPTH_COMPONENT",
        34041 => "DEPTH_STENCIL",
        6403 => "RED",
        33319 => "RG",
        36244 => "RED_INTEGER",
        33320 => "RG_INTEGER",
        36248 => "RGB_INTEGER",
        36249 => "RGBA_INTEGER",

        // Sized internal formats (8-bit)
        33321 => "R8",
        36756 => "R8_SNORM",
        33323 => "RG8",
        36757 => "RG8_SNORM",
        32849 => "RGB8",
        36758 => "RGB8_SNORM",
        36194 => "RGB565",
        32854 => "RGBA4",
        32855 => "RGB5_A1",
        32856 => "RGBA8",
        36759 => "RGBA8_SNORM",
        32857 => "RGB10_A2",
        36975 => "RGB10_A2UI",
        35905 => "SRGB8",
        35907 => "SRGB8_ALPHA8",

        // Sized float / half-float
        33325 => "R16F",
        33327 => "RG16F",
        34843 => "RGB16F",
        34842 => "RGBA16F",
        33326 => "R32F",
        33328 => "RG32F",
        34837 => "RGB32F",
        34836 => "RGBA32F",
        35898 => "R11F_G11F_B10F",
        35901 => "RGB9_E5",

        // Integer formats
        33329 => "R8I",
        33330 => "R8UI",
        33331 => "R16I",
        33332 => "R16UI",
        33333 => "R32I",
        33334 => "R32UI",
        33335 => "RG8I",
        33336 => "RG8UI",
        33337 => "RG16I",
        33338 => "RG16UI",
        33339 => "RG32I",
        33340 => "RG32UI",
        36239 => "RGB8I",
        36221 => "RGB8UI",
        36233 => "RGB16I",
        36215 => "RGB16UI",
        36227 => "RGB32I",
        36209 => "RGB32UI",
        36238 => "RGBA8I",
        36220 => "RGBA8UI",
        36226 => "RGBA16I",
        36208 => "RGBA16UI",
        36222 => "RGBA32I",
        36202 => "RGBA32UI",

        // Depth / Stencil
        33189 => "DEPTH_COMPONENT16",
        33190 => "DEPTH_COMPONENT24",
        36012 => "DEPTH_COMPONENT32F",
        35056 => "DEPTH24_STENCIL8",
        36013 => "DEPTH32F_STENCIL8",
        36168 => "STENCIL_INDEX8",

        // Compressed: ETC1
        36196 => "COMPRESSED_RGB_ETC1_WEBGL",

        // Compressed: ETC2 / EAC (WebGL 2 core, 0x9270..0x9279)
        37488 => "COMPRESSED_R11_EAC",
        37489 => "COMPRESSED_SIGNED_R11_EAC",
        37490 => "COMPRESSED_RG11_EAC",
        37491 => "COMPRESSED_SIGNED_RG11_EAC",
        37492 => "COMPRESSED_RGB8_ETC2",
        37493 => "COMPRESSED_SRGB8_ETC2",
        37494 => "COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2",
        37495 => "COMPRESSED_SRGB8_PUNCHTHROUGH_ALPHA1_ETC2",
        37496 => "COMPRESSED_RGBA8_ETC2_EAC",
        37497 => "COMPRESSED_SRGB8_ALPHA8_ETC2_EAC",

        // Compressed: S3TC / DXT
        33776 => "COMPRESSED_RGB_S3TC_DXT1_EXT",
        33777 => "COMPRESSED_RGBA_S3TC_DXT1_EXT",
        33778 => "COMPRESSED_RGBA_S3TC_DXT3_EXT",
        33779 => "COMPRESSED_RGBA_S3TC_DXT5_EXT",
        35916 => "COMPRESSED_SRGB_S3TC_DXT1_EXT",
        35917 => "COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT",
        35918 => "COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT",
        35919 => "COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT",

        // Compressed: BPTC
        36492 => "COMPRESSED_RGBA_BPTC_UNORM_EXT",
        36493 => "COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT",
        36494 => "COMPRESSED_RGB_BPTC_SIGNED_FLOAT_EXT",
        36495 => "COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT_EXT",

        // Compressed: RGTC
        36283 => "COMPRESSED_RED_RGTC1_EXT",
        36284 => "COMPRESSED_SIGNED_RED_RGTC1_EXT",
        36285 => "COMPRESSED_RED_GREEN_RGTC2_EXT",
        36286 => "COMPRESSED_SIGNED_RED_GREEN_RGTC2_EXT",

        // Compressed: ASTC (RGBA & sRGB)
        37808 => "COMPRESSED_RGBA_ASTC_4x4_KHR",
        37809 => "COMPRESSED_RGBA_ASTC_5x4_KHR",
        37810 => "COMPRESSED_RGBA_ASTC_5x5_KHR",
        37811 => "COMPRESSED_RGBA_ASTC_6x5_KHR",
        37812 => "COMPRESSED_RGBA_ASTC_6x6_KHR",
        37813 => "COMPRESSED_RGBA_ASTC_8x5_KHR",
        37814 => "COMPRESSED_RGBA_ASTC_8x6_KHR",
        37815 => "COMPRESSED_RGBA_ASTC_8x8_KHR",
        37816 => "COMPRESSED_RGBA_ASTC_10x5_KHR",
        37817 => "COMPRESSED_RGBA_ASTC_10x6_KHR",
        37818 => "COMPRESSED_RGBA_ASTC_10x8_KHR",
        37819 => "COMPRESSED_RGBA_ASTC_10x10_KHR",
        37820 => "COMPRESSED_RGBA_ASTC_12x10_KHR",
        37821 => "COMPRESSED_RGBA_ASTC_12x12_KHR",

        37840 => "COMPRESSED_SRGB8_ALPHA8_ASTC_4x4_KHR",
        37841 => "COMPRESSED_SRGB8_ALPHA8_ASTC_5x4_KHR",
        37842 => "COMPRESSED_SRGB8_ALPHA8_ASTC_5x5_KHR",
        37843 => "COMPRESSED_SRGB8_ALPHA8_ASTC_6x5_KHR",
        37844 => "COMPRESSED_SRGB8_ALPHA8_ASTC_6x6_KHR",
        37845 => "COMPRESSED_SRGB8_ALPHA8_ASTC_8x5_KHR",
        37846 => "COMPRESSED_SRGB8_ALPHA8_ASTC_8x6_KHR",
        37847 => "COMPRESSED_SRGB8_ALPHA8_ASTC_8x8_KHR",
        37848 => "COMPRESSED_SRGB8_ALPHA8_ASTC_10x5_KHR",
        37849 => "COMPRESSED_SRGB8_ALPHA8_ASTC_10x6_KHR",
        37850 => "COMPRESSED_SRGB8_ALPHA8_ASTC_10x8_KHR",
        37851 => "COMPRESSED_SRGB8_ALPHA8_ASTC_10x10_KHR",
        37852 => "COMPRESSED_SRGB8_ALPHA8_ASTC_12x10_KHR",
        37853 => "COMPRESSED_SRGB8_ALPHA8_ASTC_12x12_KHR",

        0 => "UNKNOWN",
        _ => "UNKNOWN",
    }
}

pub fn type_name(val: u32) -> &'static str {
    match val {
        5120 => "BYTE",
        5121 => "UNSIGNED_BYTE",
        5122 => "SHORT",
        5123 => "UNSIGNED_SHORT",
        5124 => "INT",
        5125 => "UNSIGNED_INT",
        5126 => "FLOAT",
        5131 => "HALF_FLOAT",
        36193 => "HALF_FLOAT_OES",
        32819 => "UNSIGNED_SHORT_4_4_4_4",
        32820 => "UNSIGNED_SHORT_5_5_5_1",
        33635 => "UNSIGNED_SHORT_5_6_5",
        33640 => "UNSIGNED_INT_2_10_10_10_REV",
        34042 => "UNSIGNED_INT_24_8",
        35899 => "UNSIGNED_INT_10F_11F_11F_REV",
        35902 => "UNSIGNED_INT_5_9_9_9_REV",
        36269 => "FLOAT_32_UNSIGNED_INT_24_8_REV",
        0 => "UNKNOWN",
        _ => "UNKNOWN",
    }
}

pub fn primitive_mode_name(val: u32) -> &'static str {
    match val {
        0 => "POINTS",
        1 => "LINES",
        2 => "LINE_LOOP",
        3 => "LINE_STRIP",
        4 => "TRIANGLES",
        5 => "TRIANGLE_STRIP",
        6 => "TRIANGLE_FAN",
        _ => "UNKNOWN",
    }
}

pub fn target_name(val: u32) -> &'static str {
    match val {
        3553 => "TEXTURE_2D",
        34067 => "TEXTURE_CUBE_MAP",
        35866 => "TEXTURE_3D",
        35864 => "TEXTURE_2D_ARRAY",
        34962 => "ARRAY_BUFFER",
        34963 => "ELEMENT_ARRAY_BUFFER",
        36009 => "FRAMEBUFFER",
        36160 => "RENDERBUFFER",
        _ => "UNKNOWN",
    }
}

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
        6408 | 36249 => 4, // RGBA, RGBA_INTEGER
        6407 | 36248 => 3, // RGB, RGB_INTEGER
        33319 | 33320 => 2, // RG, RG_INTEGER
        6406 | 6409 | 6402 | 6403 | 36244 => 1, // ALPHA, LUMINANCE, DEPTH_COMPONENT, RED, RED_INTEGER
        6410 | 34041 => 2, // LUMINANCE_ALPHA, DEPTH_STENCIL
        _ => return None,
    };

    let bytes_per_component = match type_val {
        5126 => 4, // FLOAT
        5131 | 36193 => 2, // HALF_FLOAT, HALF_FLOAT_OES
        5121 | 5120 => 1, // UNSIGNED_BYTE, BYTE
        5123 | 5122 => 2, // UNSIGNED_SHORT, SHORT
        5125 | 5124 => 4, // UNSIGNED_INT, INT
        32819 | 32820 | 33635 => return Some(2), // Packed 16-bit
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
    matches!(
        internal_format,
        34836 | 34837 | 34842 | 34843 | 33325 | 33326 | 33327 | 33328 | 35898 | 35901 | 36012 | 36013
    )
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
            let bx0 = (width + bw - 1) / bw;
            let by0 = (height + bh - 1) / bh;
            let base_bytes = (bx0 as usize) * (by0 as usize) * bpb * faces;

            // Calculate with full mip pyramid if POT or multiple levels
            let mut total_bytes = 0;
            let mut w = width;
            let mut h = height;
            let mut levels = 0;
            loop {
                let bx = (w + bw - 1) / bw;
                let by = (h + bh - 1) / bh;
                total_bytes += (bx as usize) * (by as usize) * bpb * faces;
                levels += 1;
                if w == 1 && h == 1 {
                    break;
                }
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }

            // For POT compressed textures, full mipmaps are standard.
            let chosen_bytes = if is_pot && (width > 4 || height > 4) {
                total_bytes
            } else {
                base_bytes
            };

            return (
                chosen_bytes,
                base_bytes,
                levels,
                CompressionStatus::Compressed,
                TextureEstimationMethod::FormatCalculation,
            );
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
        let chosen_bytes = if is_pot && !is_float && (width > 1 || height > 1) {
            total_bytes
        } else {
            base_bytes
        };

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
        return (
            raw_length,
            raw_length,
            1,
            CompressionStatus::Unknown,
            TextureEstimationMethod::EstimatedFallback,
        );
    }

    // 4. Fallback guess (4 B/px uncompressed assumption)
    let fallback_base = (width as usize) * (height as usize) * 4 * faces;
    (
        fallback_base,
        fallback_base,
        1,
        CompressionStatus::Unknown,
        TextureEstimationMethod::EstimatedFallback,
    )
}

