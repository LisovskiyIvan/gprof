use crate::model::ParsedCapture;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneTreeAnalysisResult {
    pub total_nodes: usize,
    pub nodes: Vec<SceneNodeItem>,
    pub categories_summary: Vec<NodeCategorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNodeItem {
    pub id: usize,
    pub name: String,
    pub category: String,
    pub world_position: Option<[f32; 3]>,
    pub draw_count: usize,
    pub total_triangles: usize,
    pub vertex_buffer_bytes: usize,
    pub shaders: Vec<String>,
    pub passes_used: Vec<usize>,
    pub passes_names: Vec<String>,
    pub samplers: Vec<String>,
    pub is_instanced: bool,
    pub instance_count: usize,
    pub uses_32bit_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCategorySummary {
    pub category: String,
    pub node_count: usize,
    pub draw_count: usize,
    pub triangle_count: usize,
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum NodeKey {
    WorldPos([i32; 3], usize), // (x*10, y*10, z*10), approx vertex buffer size / 1024
    Particles,
    PostProcess(String),
    LayerUI,
    Instanced(usize), // vertex_bytes
    Generic(String),  // shader name
}

pub fn analyze_scene_tree(capture: &ParsedCapture) -> SceneTreeAnalysisResult {
    // Map draws to pass index
    let mut draw_pass_map: Vec<(usize, String)> = Vec::with_capacity(capture.draws.len());
    let mut cursor = 0;
    for (p_idx, pass) in capture.passes.iter().enumerate() {
        for _ in 0..pass.draw_count {
            if cursor < capture.draws.len() {
                draw_pass_map.push((p_idx, pass.fbo_name.clone()));
                cursor += 1;
            }
        }
    }
    while draw_pass_map.len() < capture.draws.len() {
        draw_pass_map.push((0, "Default Backbuffer".to_string()));
    }

    let mut groups: FxHashMap<NodeKey, Vec<usize>> = FxHashMap::default();

    for (d_idx, draw) in capture.draws.iter().enumerate() {
        let s_lower = draw.shader_name.to_lowercase();

        let key = if s_lower.contains("particle") {
            NodeKey::Particles
        } else if s_lower.contains("layer") {
            NodeKey::LayerUI
        } else if draw.triangles == 2 && (s_lower.contains("blur") || s_lower.contains("merge") || s_lower.contains("procedural")) {
            NodeKey::PostProcess(draw.shader_name.clone())
        } else if let Some(pos) = draw.world_position {
            let px = (pos[0] * 10.0).round() as i32;
            let py = (pos[1] * 10.0).round() as i32;
            let pz = (pos[2] * 10.0).round() as i32;
            let v_bucket = draw.vertex_buffer_bytes / 1024;
            NodeKey::WorldPos([px, py, pz], v_bucket)
        } else if draw.is_instanced {
            NodeKey::Instanced(draw.vertex_buffer_bytes)
        } else {
            NodeKey::Generic(draw.shader_name.clone())
        };

        groups.entry(key).or_default().push(d_idx);
    }

    let mut nodes = Vec::new();
    let mut node_counter = 1;

    for (key, draw_indices) in groups {
        let mut total_triangles = 0;
        let mut max_vram = 0;
        let mut shaders = Vec::new();
        let mut passes_used = Vec::new();
        let mut passes_names = Vec::new();
        let mut samplers = Vec::new();
        let mut is_instanced = false;
        let mut max_instances: usize = 1;
        let mut uses_32bit_index = false;
        let mut world_pos = None;

        for &idx in &draw_indices {
            let d = &capture.draws[idx];
            total_triangles += d.triangles;
            if d.vertex_buffer_bytes > max_vram {
                max_vram = d.vertex_buffer_bytes;
            }
            if !shaders.contains(&d.shader_name) {
                shaders.push(d.shader_name.clone());
            }
            if idx < draw_pass_map.len() {
                let (pidx, ref pname) = draw_pass_map[idx];
                if !passes_used.contains(&pidx) {
                    passes_used.push(pidx);
                    passes_names.push(pname.clone());
                }
            }
            for samp in &d.samplers {
                if !samplers.contains(samp) {
                    samplers.push(samp.clone());
                }
            }
            if d.is_instanced {
                is_instanced = true;
                if (d.instance_count as usize) > max_instances {
                    max_instances = d.instance_count as usize;
                }
            }
            if let Some(ref itype) = d.index_type_name {
                if itype.contains("32-bit") {
                    uses_32bit_index = true;
                }
            }
            if world_pos.is_none() && d.world_position.is_some() {
                world_pos = d.world_position;
            }
        }

        // Determine name and category
        let (category, name) = match key {
            NodeKey::Particles => (
                "GPU Particle System".to_string(),
                "GPU Transform Feedback Particle Emitters".to_string(),
            ),
            NodeKey::LayerUI => (
                "UI / 2D Layer".to_string(),
                "Screen Space UI & Canvas Layer".to_string(),
            ),
            NodeKey::PostProcess(ref sname) => (
                "Post-Process Effect".to_string(),
                format!("{} (Fullscreen Filter Quad)", sname),
            ),
            NodeKey::Instanced(bytes) => (
                "Instanced Mesh Group".to_string(),
                format!("Instanced Environment Props ({:.1} KB buffer)", bytes as f64 / 1024.0),
            ),
            NodeKey::WorldPos([px, py, pz], _) => {
                let x = px as f32 / 10.0;
                let y = py as f32 / 10.0;
                let z = pz as f32 / 10.0;
                let is_vat = samplers.iter().any(|s| {
                    let sl = s.to_lowercase();
                    sl.contains("vat") || sl.contains("bone")
                });

                if is_vat {
                    (
                        "Character (VAT Animated)".to_string(),
                        format!("VAT Animated Character @ ({:.1}, {:.1}, {:.1})", x, y, z),
                    )
                } else if uses_32bit_index || total_triangles >= 100_000 {
                    (
                        "Hero Mesh (32-Bit Index)".to_string(),
                        format!("Hero Mesh @ ({:.1}, {:.1}, {:.1})", x, y, z),
                    )
                } else if total_triangles >= 20_000 {
                    (
                        "Medium-Poly Geometry".to_string(),
                        format!("Environment Model @ ({:.1}, {:.1}, {:.1})", x, y, z),
                    )
                } else {
                    (
                        "Static Scene Prop".to_string(),
                        format!("Static Prop @ ({:.1}, {:.1}, {:.1})", x, y, z),
                    )
                }
            }
            NodeKey::Generic(ref sname) => (
                "Scene Geometry".to_string(),
                format!("Submesh (Shader: {})", sname),
            ),
        };

        nodes.push(SceneNodeItem {
            id: node_counter,
            name,
            category,
            world_position: world_pos,
            draw_count: draw_indices.len(),
            total_triangles,
            vertex_buffer_bytes: max_vram,
            shaders,
            passes_used,
            passes_names,
            samplers,
            is_instanced,
            instance_count: max_instances,
            uses_32bit_index,
        });

        node_counter += 1;
    }

    nodes.sort_by(|a, b| b.total_triangles.cmp(&a.total_triangles).then_with(|| b.draw_count.cmp(&a.draw_count)));

    // Categorical summary
    let mut cat_map: FxHashMap<String, (usize, usize, usize)> = FxHashMap::default();
    for node in &nodes {
        let entry = cat_map.entry(node.category.clone()).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += node.draw_count;
        entry.2 += node.total_triangles;
    }

    let mut categories_summary: Vec<NodeCategorySummary> = cat_map
        .into_iter()
        .map(|(category, (node_count, draw_count, triangle_count))| NodeCategorySummary {
            category,
            node_count,
            draw_count,
            triangle_count,
        })
        .collect();
    categories_summary.sort_by(|a, b| b.triangle_count.cmp(&a.triangle_count));

    SceneTreeAnalysisResult {
        total_nodes: nodes.len(),
        nodes,
        categories_summary,
    }
}
