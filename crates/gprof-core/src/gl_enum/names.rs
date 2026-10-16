//! WebGL constants, enum names, and memory calculation helpers.

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
