//! WebGL constants and helper formatting.

pub fn format_name(val: u32) -> &'static str {
    match val {
        6406 => "ALPHA",
        6407 => "RGB",
        6408 => "RGBA",
        6409 => "LUMINANCE",
        6410 => "LUMINANCE_ALPHA",
        6402 => "DEPTH_COMPONENT",
        34041 => "DEPTH_STENCIL",
        32856 => "RGBA8",
        32854 => "RGB8",
        35907 => "SRGB8_ALPHA8",
        35905 => "SRGB8",
        34836 => "RGBA32F",
        34837 => "RGB32F",
        34842 => "RGBA16F",
        34843 => "RGB16F",
        37492 => "SRGB8_ALPHA8_ASTC/ETC",
        37496 => "COMPRESSED_RGBA8_ETC2_EAC",
        37808 => "COMPRESSED_RGBA_ASTC_4x4",
        33776 => "COMPRESSED_RGB_S3TC_DXT1",
        33777 => "COMPRESSED_RGBA_S3TC_DXT5",
        33779 => "COMPRESSED_RGBA_S3TC_DXT3",
        36194 => "RGBA32UI",
        36208 => "RGBA16UI",
        36220 => "RGBA8UI",
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
        36193 => "HALF_FLOAT",
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
