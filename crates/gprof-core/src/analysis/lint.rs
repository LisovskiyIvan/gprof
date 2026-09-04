use crate::analysis::ComprehensiveAnalysis;
use crate::model::ParsedCapture;
use crate::report::{format_bytes, format_num};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LintSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl LintSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            LintSeverity::Critical => "CRITICAL",
            LintSeverity::High => "HIGH",
            LintSeverity::Medium => "MEDIUM",
            LintSeverity::Low => "LOW",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            LintSeverity::Critical => "🔴",
            LintSeverity::High => "🟠",
            LintSeverity::Medium => "🟡",
            LintSeverity::Low => "ℹ️",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintRule {
    pub id: String,
    pub severity: LintSeverity,
    pub title: String,
    pub impact: String,
    pub description: String,
    pub code_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintReport {
    pub total_issues: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub rules: Vec<LintRule>,
}

pub fn run_linter(capture: &ParsedCapture, analysis: &ComprehensiveAnalysis) -> LintReport {
    let mut rules = Vec::new();

    // 1. GlowLayer Occluders
    if analysis.glow.black_occluder_draws > 0 {
        rules.push(LintRule {
            id: "BABYLON-001".to_string(),
            severity: LintSeverity::Critical,
            title: "GlowLayer Renders Non-Glowing Geometry as Black Occluders".to_string(),
            impact: format!(
                "{} draw calls and {} triangles ({:.1}% of glow pass) wasted on black occluders.",
                analysis.glow.black_occluder_draws,
                format_num(analysis.glow.black_occluder_triangles),
                analysis.glow.wasted_triangle_percentage
            ),
            description: "By default in Babylon.js, GlowLayer renders all meshes in the scene with pure black [0,0,0,1] to occlude glow behind them. Switching GlowLayer to whitelist mode renders only active glowing meshes, eliminating all occluder draw calls.".to_string(),
            code_fix: r#"// Switch GlowLayer from blacklist to whitelist mode:
const glowLayer = new BABYLON.GlowLayer("glow", scene);
// Whitelist only meshes that actually have emissive/glow materials:
glowingMeshes.forEach(mesh => glowLayer.addIncludedOnlyMesh(mesh));"#.to_string(),
        });
    }

    // 2. Thin Instancing Candidates
    if analysis.instancing.total_potential_saved_draws > 5 {
        rules.push(LintRule {
            id: "BABYLON-002".to_string(),
            severity: LintSeverity::High,
            title: "Duplicate Geometry Rendered in Separate Draw Calls".to_string(),
            impact: format!(
                "Can eliminate up to {} draw calls per frame across {} candidate groups.",
                analysis.instancing.total_potential_saved_draws,
                analysis.instancing.total_candidates_groups
            ),
            description: "Found multiple draw calls sharing identical vertex buffers and shaders. Merging these into Babylon.js Thin Instances batches them into a single instanced draw call on the GPU.".to_string(),
            code_fix: r#"// Combine duplicate meshes into Babylon.js Thin Instances:
const matrices = new Float32Array(instanceCount * 16);
instances.forEach((inst, i) => inst.matrix.copyToArray(matrices, i * 16));
masterMesh.thinInstanceSetBuffer("matrix", matrices, 16, false);"#.to_string(),
        });
    }

    // 3. Texture Compression (KTX2 / Basis)
    if analysis.compression.estimated_ktx2_savings_bytes > 50 * 1024 * 1024 {
        rules.push(LintRule {
            id: "BABYLON-003".to_string(),
            severity: LintSeverity::High,
            title: "Uncompressed Textures Bloat GPU VRAM".to_string(),
            impact: format!(
                "Save up to {} VRAM by converting uncompressed RGBA8 textures to KTX2.",
                format_bytes(analysis.compression.estimated_ktx2_savings_bytes)
            ),
            description: "Textures are uploaded as uncompressed RGBA8 (4 bytes per pixel). Using KTX2 with ASTC/BC7/ETC2 reduces VRAM to 1 byte per pixel, cuts GPU bandwidth, and avoids CPU texture decompression.".to_string(),
            code_fix: r#"// Configure Babylon.js KTX2 Transcoder:
BABYLON.KhronosTextureContainer2.URLConfig = {
    jsDecoderModule: "https://cdn.babylonjs.com/babylon.ktx2Decoder.js",
    wasmUASTCToASTC: "https://cdn.babylonjs.com/ktx2Transcoders/1/uastc_astc.wasm",
    wasmBC7: "https://cdn.babylonjs.com/ktx2Transcoders/1/uastc_bc7.wasm"
};"#.to_string(),
        });
    }

    // 4. Skybox Oversized Textures
    let skybox_8k = capture.textures.iter().filter(|t| t.width >= 8192 || t.height >= 8192).count();
    if skybox_8k > 0 {
        rules.push(LintRule {
            id: "BABYLON-004".to_string(),
            severity: LintSeverity::High,
            title: "Oversized 8K Skybox Textures".to_string(),
            impact: format!("{} textures are 8K (>=8192px), taking hundreds of megabytes of VRAM.", skybox_8k),
            description: "8192x2048 textures consume 64 MB uncompressed VRAM per texture. Downscaling background skyboxes to 2048px preserves visual fidelity while saving up to 80% VRAM.".to_string(),
            code_fix: r#"// Downscale skybox cubemap/equirectangular textures to 2K:
const skyboxTexture = new BABYLON.CubeTexture("textures/skybox_2k", scene);"#.to_string(),
        });
    }

    // 5. Duplicate bufferSubData Uploads
    if analysis.buffers.duplicate_dynamic_uploads > 0 {
        rules.push(LintRule {
            id: "BABYLON-005".to_string(),
            severity: LintSeverity::Medium,
            title: "Duplicate Dynamic Buffer Uploads (bufferSubData)".to_string(),
            impact: format!("{} calls uploaded identical data to the GPU in the same frame.", analysis.buffers.duplicate_dynamic_uploads),
            description: "DThinInstanceManager or updateDynamicVertexBuffer is repeatedly uploading identical float matrix data. Add a dirty check before issuing updates to dynamic vertex buffers.".to_string(),
            code_fix: r#"// In DThinInstanceManager, guard buffer update with dirty flag:
if (this._isDirty) {
    this.flushDirty();
    this._isDirty = false;
}"#.to_string(),
        });
    }

    // 6. Per-Frame texImage2D Reallocations
    if analysis.uploads.total_uploads > 0 {
        rules.push(LintRule {
            id: "BABYLON-006".to_string(),
            severity: LintSeverity::Medium,
            title: "Per-Frame Dynamic texImage2D Texture Reallocations".to_string(),
            impact: format!("{} calls to texImage2D per frame reallocate GPU storage.", analysis.uploads.total_uploads),
            description: "texImage2D destroys and reallocates texture storage on the driver. If updating an 8x1 float texture every frame, use texSubImage2D or pass data via uniforms.".to_string(),
            code_fix: r#"// Instead of dynamic texture reallocation, pass playback data via uniform:
material.setColor4("playbackData", new BABYLON.Color4(v0, v1, v2, v3));
// Or update existing texture without reallocation:
gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, width, height, format, type, data);"#.to_string(),
        });
    }

    // 7. 32-Bit Index Buffer Meshes
    if analysis.buffers.index_32bit_draws > 0 {
        rules.push(LintRule {
            id: "BABYLON-007".to_string(),
            severity: LintSeverity::Medium,
            title: "Meshes with >65,535 Vertices Force 32-Bit Index Buffers".to_string(),
            impact: format!("{} draw calls use UNSIGNED_INT, triggering VERTEX_PULLING_INDEX_BUFFER_32BITS shader variants.", analysis.buffers.index_32bit_draws),
            description: "Meshes with >65k indices require 32-bit index buffers (UNSIGNED_INT), which doubles IBO bandwidth on mobile GPUs and forces Babylon.js to compile special shader variants.".to_string(),
            code_fix: r#"// Subdivide meshes with >65k vertices in Babylon.js or 3D modeling tool:
mesh.subdivide(Math.ceil(mesh.getTotalIndices() / 60000));"#.to_string(),
        });
    }

    // 8. Shader Ping-Pong Switching
    if analysis.sorting.ping_pong_switches > 5 {
        rules.push(LintRule {
            id: "BABYLON-008".to_string(),
            severity: LintSeverity::Medium,
            title: "Material Sorting Failure: Ping-Pong Shader Switching".to_string(),
            impact: format!("{} ping-pong switches (A -> B -> A), causing {} redundant useProgram switches.", analysis.sorting.ping_pong_switches, analysis.sorting.redundant_switches),
            description: "Draw calls are not grouped by material/shader. Sorting opaque draw calls by material ID allows the driver to keep the shader program and textures bound across multiple meshes.".to_string(),
            code_fix: r#"// Sort opaque meshes by material uniqueId to minimize shader switches:
scene.setRenderingOrder(0, BABYLON.RenderingGroup.AUTOCLEAR, (a, b) => {
    return (a.material?.uniqueId || 0) - (b.material?.uniqueId || 0);
});"#.to_string(),
        });
    }

    // 9. Opaque Draws Executed After Transparent Draws
    if analysis.sorting.total_opaque_after_blended > 0 {
        rules.push(LintRule {
            id: "BABYLON-009".to_string(),
            severity: LintSeverity::Medium,
            title: "Opaque Meshes Drawn After Transparent Meshes".to_string(),
            impact: format!("{} opaque draw calls executed after blend started, breaking Early-Z rejection.", analysis.sorting.total_opaque_after_blended),
            description: "Opaque meshes should always render before transparent/blended meshes. Interleaving opaque draws after blending causes overdraw and forces extra depth/blend state toggles.".to_string(),
            code_fix: r#"// Assign transparent meshes to a distinct rendering group:
opaqueMesh.renderingGroupId = 0;      // Rendered first (Early-Z active)
transparentMesh.renderingGroupId = 1; // Rendered second (Back-to-front)"#.to_string(),
        });
    }

    // 10. Redundant depthMask State Calls
    if capture.state_changes.redundant_depth_mask > 10 {
        rules.push(LintRule {
            id: "BABYLON-010".to_string(),
            severity: LintSeverity::Low,
            title: "Redundant depthMask Calls Bypass ThinEngine State Cache".to_string(),
            impact: format!("{} redundant depthMask() calls stall the WebGL pipeline.", capture.state_changes.redundant_depth_mask),
            description: "Calling gl.depthMask() directly or toggling depth write without checking current state causes redundant driver calls. Use engine.setDepthWrite() to leverage Babylon.js state caching.".to_string(),
            code_fix: r#"// Use Babylon.js engine state cache instead of raw WebGL calls:
engine.setDepthWrite(false); // Only issues driver call if state changes"#.to_string(),
        });
    }

    let critical = rules.iter().filter(|r| r.severity == LintSeverity::Critical).count();
    let high = rules.iter().filter(|r| r.severity == LintSeverity::High).count();
    let medium = rules.iter().filter(|r| r.severity == LintSeverity::Medium).count();
    let low = rules.iter().filter(|r| r.severity == LintSeverity::Low).count();

    LintReport {
        total_issues: rules.len(),
        critical_count: critical,
        high_count: high,
        medium_count: medium,
        low_count: low,
        rules,
    }
}

pub fn format_lint_report(report: &LintReport) -> String {
    let mut out = String::new();
    out.push_str("# Babylon.js Performance Lint Report\n\n");

    out.push_str(&format!(
        "Summary: **{}** issues found (🔴 {} Critical, 🟠 {} High, 🟡 {} Medium, ℹ️ {} Low)\n\n",
        report.total_issues,
        report.critical_count,
        report.high_count,
        report.medium_count,
        report.low_count
    ));

    for r in &report.rules {
        out.push_str(&format!("## {} [{}] {}\n\n", r.severity.emoji(), r.id, r.title));
        out.push_str(&format!("- **Severity:** {}\n", r.severity.as_str()));
        out.push_str(&format!("- **Impact:** {}\n", r.impact));
        out.push_str(&format!("- **Description:** {}\n\n", r.description));
        out.push_str("### Recommended Code Fix (TypeScript):\n\n```typescript\n");
        out.push_str(&r.code_fix);
        out.push_str("\n```\n\n---\n\n");
    }

    out
}
