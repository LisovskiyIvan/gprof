use crate::analysis::ComprehensiveAnalysis;
use crate::model::{ParsedCapture, TextureUsageCategory};
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
            severity: LintSeverity::Medium,
            title: "GlowLayer Evaluates Non-Emissive Geometry for Occlusion Masking".to_string(),
            impact: format!(
                "[Hypothesis / Candidate] {} occluder draw calls ({} triangles, {:.1}% of glow pass) mask background glow. {} active glowing draws.",
                analysis.glow.black_occluder_draws,
                format_num(analysis.glow.black_occluder_triangles),
                analysis.glow.occluder_triangle_percentage,
                analysis.glow.active_glowing_draws
            ),
            description: "In Babylon.js, GlowLayer renders non-emissive meshes in black to write depth and prevent glow from bleeding through foreground objects. Verify whether distant background meshes or non-occluding objects can be excluded using custom include/exclude lists to reduce pass draw calls.".to_string(),
            code_fix: r#"// Configure GlowLayer inclusion/exclusion list:
const glowLayer = new BABYLON.GlowLayer("glow", scene);
// Option A: Explicit whitelist of only glowing meshes
glowingMeshes.forEach(m => glowLayer.addIncludedOnlyMesh(m));
// Option B: Exclude distant background meshes from occluding
backgroundMeshes.forEach(m => glowLayer.addExcludedMesh(m));"#.to_string(),
        });
    }

    // 2. Instancing Candidates (Pass-Isolated)
    if analysis.instancing.total_potential_saved_draws > 5 {
        rules.push(LintRule {
            id: "BABYLON-002".to_string(),
            severity: LintSeverity::Medium,
            title: "Duplicate Geometry Within Render Passes (Instancing Candidates)".to_string(),
            impact: format!(
                "[Hypothesis / Candidate] Theoretical upper bound: up to {} draw calls could be merged across {} candidate groups within their respective passes.",
                analysis.instancing.total_potential_saved_draws,
                analysis.instancing.total_candidates_groups
            ),
            description: "Multiple draw calls within individual render passes share identical vertex buffer allocations and shaders. Merging them into Babylon.js Thin Instances batches them into a single instanced draw call per pass (feasibility depends on dynamic uniform variation and culling).".to_string(),
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
            severity: LintSeverity::Medium,
            title: "Uncompressed Color Textures (KTX2 Compression Candidates)".to_string(),
            impact: format!(
                "[Calculated Estimate] Theoretical upper bound: converting uncompressed color textures could reduce VRAM by up to {} (~75% uncompressed color memory).",
                format_bytes(analysis.compression.estimated_ktx2_savings_bytes)
            ),
            description: "Color textures uploaded as uncompressed RGBA8 consume 4 bytes per pixel. Block compression (ASTC / BC7 / ETC2 via KTX2) reduces memory to ~1 byte per pixel. GPU animation/VAT data textures and already compressed textures are excluded.".to_string(),
            code_fix: r#"// Configure Babylon.js KTX2 Transcoder:
BABYLON.KhronosTextureContainer2.URLConfig = {
    jsDecoderModule: "https://cdn.babylonjs.com/babylon.ktx2Decoder.js",
    wasmUASTCToASTC: "https://cdn.babylonjs.com/ktx2Transcoders/1/uastc_astc.wasm",
    wasmBC7: "https://cdn.babylonjs.com/ktx2Transcoders/1/uastc_bc7.wasm"
};"#.to_string(),
        });
    }

    // 4. Oversized 8K Color Textures (excluding VAT and 1D LUTs)
    let color_8k = capture.textures.iter().filter(|t| t.category == TextureUsageCategory::ColorMaterial && (t.width >= 8192 || t.height >= 8192) && t.width.min(t.height) >= 512).count();
    if color_8k > 0 {
        rules.push(LintRule {
            id: "BABYLON-004".to_string(),
            severity: LintSeverity::Medium,
            title: "Oversized 8K Color / Skybox Textures".to_string(),
            impact: format!("[Calculated Estimate] {} color textures have dimensions >= 8192px.", color_8k),
            description: "High-resolution 8K color textures take substantial VRAM. Verifying whether 4K or 2K downscaling preserves sufficient visual fidelity can reduce memory pressure.".to_string(),
            code_fix: r#"// Downscale background skybox or environment textures to 2K/4K:
const skyboxTexture = new BABYLON.CubeTexture("textures/skybox_2k", scene);"#.to_string(),
        });
    }

    // 5. Duplicate bufferSubData Uploads
    if analysis.buffers.duplicate_dynamic_uploads > 0 {
        rules.push(LintRule {
            id: "BABYLON-005".to_string(),
            severity: LintSeverity::Medium,
            title: "Duplicate Dynamic Buffer Uploads (bufferSubData)".to_string(),
            impact: format!("[Observed Fact] {} bufferSubData calls uploaded identical data to the driver in the same frame.", analysis.buffers.duplicate_dynamic_uploads),
            description: "Dynamic vertex or transform feedback buffers are repeatedly uploaded with identical data. Guard buffer updates with dirty flags to eliminate redundant CPU-to-driver copies.".to_string(),
            code_fix: r#"// Guard dynamic buffer updates with a dirty flag:
if (this._isDirty) {
    this.flushDirty();
    this._isDirty = false;
}"#.to_string(),
        });
    }

    // 6. Dynamic texImage2D Allocations
    if analysis.uploads.total_uploads > 0 {
        rules.push(LintRule {
            id: "BABYLON-006".to_string(),
            severity: LintSeverity::Low,
            title: "Dynamic texImage2D Texture Allocation Detected".to_string(),
            impact: format!("[Observed Fact] {} calls to texImage2D observed during capture.", analysis.uploads.total_uploads),
            description: "texImage2D allocates driver texture storage. For initial scene asset loading, this is expected behavior; if occurring continuously in runtime animation loops, texSubImage2D or uniform arrays should be preferred to avoid GPU reallocation overhead.".to_string(),
            code_fix: r#"// For runtime updates without reallocation, use texSubImage2D:
gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, width, height, format, type, data);
// Or pass dynamic scalar/vector parameters via shader uniforms:
material.setColor4("playbackData", new BABYLON.Color4(v0, v1, v2, v3));"#.to_string(),
        });
    }

    // 7. 32-Bit Index Buffer Meshes
    if analysis.buffers.index_32bit_draws > 0 {
        rules.push(LintRule {
            id: "BABYLON-007".to_string(),
            severity: LintSeverity::Low,
            title: "32-Bit Index Buffers (UNSIGNED_INT)".to_string(),
            impact: format!("[Observed Fact] {} draw calls use 32-bit index buffers (UNSIGNED_INT).", analysis.buffers.index_32bit_draws),
            description: "32-bit index buffers are fully supported across all modern WebGL 2 devices and browsers. Splitting meshes into <= 65,535 vertices to use 16-bit indices (UNSIGNED_SHORT) is an optional candidate for extreme memory and bandwidth optimization on entry-level mobile devices.".to_string(),
            code_fix: r#"// Optional: Subdivide large meshes into <= 65k vertices for 16-bit indices:
mesh.subdivide(Math.ceil(mesh.getTotalIndices() / 60000));"#.to_string(),
        });
    }

    // 8. Shader Ping-Pong Switching
    if analysis.sorting.ping_pong_switches > 5 {
        rules.push(LintRule {
            id: "BABYLON-008".to_string(),
            severity: LintSeverity::Medium,
            title: "Material Sorting: Ping-Pong Shader Switching".to_string(),
            impact: format!("[Calculated Estimate] {} ping-pong switches (A -> B -> A), causing up to {} reducible useProgram switches within phases.", analysis.sorting.ping_pong_switches, analysis.sorting.reducible_switches),
            description: "Draw calls within render phases alternate between different shaders. Sorting opaque draw calls by material ID allows the driver to keep shader programs bound across multiple meshes.".to_string(),
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
            severity: LintSeverity::Low,
            title: "Opaque Draws Executed After Blend Phase".to_string(),
            impact: format!("[Hypothesis / Candidate] {} opaque draw calls were submitted after alpha-blended draws in the same pass.", analysis.sorting.total_opaque_after_blended),
            description: "If these draw calls represent 2D UI elements, overlays, or depth-untested decals, this ordering is intentional. If they represent 3D scene geometry, moving them prior to the alpha-blend phase enables hardware Early-Z depth rejection.".to_string(),
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
            title: "Redundant depthMask State Calls".to_string(),
            impact: format!("[Observed Fact] {} depthMask() calls repeated the currently active depth write state.", capture.state_changes.redundant_depth_mask),
            description: "Calling gl.depthMask() with the existing state introduces redundant driver calls. When using custom multipass rendering or overlays, ensure state toggles leverage engine state caching.".to_string(),
            code_fix: r#"// Leverage Babylon.js engine state cache instead of raw WebGL calls:
engine.setDepthWrite(false); // Only issues driver call if state actually changes"#.to_string(),
        });
    }

    // 11. NPOT Textures
    if analysis.compression.npot_count > 0 {
        rules.push(LintRule {
            id: "BABYLON-011".to_string(),
            severity: LintSeverity::Low,
            title: "Non-Power-of-Two (NPOT) Textures".to_string(),
            impact: format!("[Observed Fact] {} non-data textures have Non-Power-of-Two dimensions.", analysis.compression.npot_count),
            description: "In WebGL 2, NPOT textures are fully compliant, supporting mipmapping and REPEAT/CLAMP_TO_EDGE wrap modes. If targeting WebGL 1 fallback, ensure textures use CLAMP_TO_EDGE and no mipmaps.".to_string(),
            code_fix: r#"// For WebGL 1 fallback compatibility, configure CLAMP_TO_EDGE:
texture.wrapU = BABYLON.Texture.CLAMP_ADDRESSMODE;
texture.wrapV = BABYLON.Texture.CLAMP_ADDRESSMODE;"#.to_string(),
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
    out.push_str("# Babylon.js & WebGL Detected Patterns Audit\n\n");

    out.push_str(&format!(
        "Summary: **{}** patterns identified (🔴 {} Critical, 🟠 {} High, 🟡 {} Medium, ℹ️ {} Low)\n\n",
        report.total_issues,
        report.critical_count,
        report.high_count,
        report.medium_count,
        report.low_count
    ));

    out.push_str("| Rule ID | Severity | Title | Detected Telemetry / Metric | Description |\n");
    out.push_str("|---|---|---|---|---|\n");
    for r in &report.rules {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            r.id,
            r.severity.as_str(),
            r.title,
            r.impact,
            r.description
        ));
    }
    out.push_str("\n");

    out
}

