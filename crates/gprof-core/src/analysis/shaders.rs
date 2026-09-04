use crate::model::*;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PbrVariantDiff {
    pub program_id: u32,
    pub draw_count: usize,
    pub unique_defines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadersAnalysisResult {
    pub total_programs: usize,
    pub pbr_variants_count: usize,
    pub use_program_calls: usize,
    pub redundant_use_program_calls: usize,
    pub pbr_common_defines_count: usize,
    pub pbr_variants: Vec<PbrVariantDiff>,
    pub fragmentation_reasons: Vec<String>,
}

pub fn analyze_shaders(programs: &[ProgramItem], use_program_calls: usize, redundant_calls: usize) -> ShadersAnalysisResult {
    let total_programs = programs.len();
    let pbr_progs: Vec<&ProgramItem> = programs.iter().filter(|p| p.is_pbr).collect();
    let pbr_variants_count = pbr_progs.len();

    let mut common_defines: Option<FxHashSet<String>> = None;
    for p in &pbr_progs {
        let set: FxHashSet<String> = p.defines.iter().cloned().collect();
        match &mut common_defines {
            None => common_defines = Some(set),
            Some(curr) => {
                *curr = curr.intersection(&set).cloned().collect();
            }
        }
    }
    let common_set = common_defines.unwrap_or_default();
    let pbr_common_defines_count = common_set.len();

    let mut variants = Vec::new();
    let mut has_index_32 = false;
    let mut has_roughness_alpha = false;
    let mut has_roughness_green = false;
    let mut has_alpha_test_diff = false;
    let mut has_uv_diff = false;
    let mut has_vat_diff = false;

    for p in &pbr_progs {
        let mut diff: Vec<String> = p.defines.iter().filter(|d| !common_set.contains(*d)).cloned().collect();
        diff.sort();

        for d in &diff {
            if d.contains("VERTEX_PULLING_INDEX_BUFFER_32BITS") {
                has_index_32 = true;
            }
            if d.contains("ROUGHNESSSTOREINMETALMAPALPHA") {
                has_roughness_alpha = true;
            }
            if d.contains("ROUGHNESSSTOREINMETALMAPGREEN") {
                has_roughness_green = true;
            }
            if d.contains("ALPHATESTVALUE") {
                has_alpha_test_diff = true;
            }
            if d.contains("ALBEDODIRECTUV") || d.contains("UV2") {
                has_uv_diff = true;
            }
            if d.contains("GPU_VAT_PALETTE") || d.contains("BONETEXTURE") {
                has_vat_diff = true;
            }
        }

        variants.push(PbrVariantDiff {
            program_id: p.id,
            draw_count: p.draw_count,
            unique_defines: diff,
        });
    }

    variants.sort_by(|a, b| b.draw_count.cmp(&a.draw_count));

    let mut reasons = Vec::new();
    if has_index_32 {
        reasons.push("32-bit vs 16-bit Index Buffers: meshes with >65k indices trigger VERTEX_PULLING_INDEX_BUFFER_32BITS variant.".to_string());
    }
    if has_roughness_alpha && has_roughness_green {
        reasons.push("Inconsistent ORM Packing: some materials store Roughness in Alpha, others in Green.".to_string());
    }
    if has_alpha_test_diff {
        reasons.push("Alpha Test Threshold Divergence: materials have different ALPHATESTVALUE (e.g. 0.4 vs 0.05).".to_string());
    }
    if has_uv_diff {
        reasons.push("UV Channel Mismatch: some meshes use UV channel 0, others UV channel 1 or UV2.".to_string());
    }
    if has_vat_diff {
        reasons.push("Skeletal / VAT Animation: animated meshes compile with GPU_VAT_PALETTE / BONETEXTURE while static meshes do not.".to_string());
    }

    ShadersAnalysisResult {
        total_programs,
        pbr_variants_count,
        use_program_calls,
        redundant_use_program_calls: redundant_calls,
        pbr_common_defines_count,
        pbr_variants: variants,
        fragmentation_reasons: reasons,
    }
}
