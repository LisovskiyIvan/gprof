//! Typed command view: only the fields the state machine reads.
//! Heavy subtrees Spector embeds per draw (`uniforms[].texture.visual`
//! base64 previews, `shaders[].translatedSource`) are never inspected, so
//! they are skipped instead of being materialized as `serde_json::Value`.

use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;

use super::helpers::{collect_textures, RawTexData};

#[derive(Deserialize)]
pub(super) struct Cmd<'a> {
    #[serde(borrow, default)]
    pub name: &'a str,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default, rename = "commandArguments")]
    pub command_arguments: Option<Vec<Value>>,
    #[serde(default, rename = "stackTrace")]
    pub stack_trace: Option<Vec<String>>,
    #[serde(borrow, default, rename = "DrawCall")]
    pub draw_call: Option<DrawCall<'a>>,
    #[serde(default, rename = "VisualState")]
    pub visual_state: Option<Value>,
}

impl<'a> Cmd<'a> {
    pub fn stack_trace(&self) -> Vec<String> {
        self.stack_trace.clone().unwrap_or_default()
    }

    /// Texture discovery equivalent to walking the whole raw command object.
    pub fn collect_textures(&self, map: &mut FxHashMap<u32, RawTexData>) {
        if let Some(v) = &self.visual_state {
            collect_textures(v, map);
        }
        if let Some(args) = &self.command_arguments {
            for v in args {
                collect_textures(v, map);
            }
        }
        if let Some(dc) = &self.draw_call {
            dc.collect_textures(map);
        }
    }
}

#[derive(Deserialize)]
pub(super) struct DrawCall<'a> {
    #[serde(default)]
    pub shaders: Option<Vec<Shader>>,
    #[serde(borrow, default)]
    pub uniforms: Option<Vec<Uniform<'a>>>,
    #[serde(borrow, default)]
    pub attributes: Option<Vec<Attribute<'a>>>,
    #[serde(default, rename = "elementArray")]
    pub element_array: Option<Value>,
    #[serde(default, rename = "frameBuffer")]
    pub frame_buffer: Option<Value>,
    #[serde(default, rename = "transformFeedbacks")]
    pub transform_feedbacks: Option<Value>,
    #[serde(default, rename = "uniformBlocks")]
    pub uniform_blocks: Option<Value>,
    #[serde(default, rename = "programStatus")]
    pub program_status: Option<Value>,
}

impl DrawCall<'_> {
    fn collect_textures(&self, map: &mut FxHashMap<u32, RawTexData>) {
        for v in [&self.element_array, &self.frame_buffer, &self.transform_feedbacks, &self.uniform_blocks, &self.program_status]
            .into_iter()
            .flatten()
        {
            collect_textures(v, map);
        }
        if let Some(attrs) = &self.attributes {
            for a in attrs {
                if let Some(v) = &a.buffer_binding {
                    collect_textures(v, map);
                }
            }
        }
        if let Some(uniforms) = &self.uniforms {
            for u in uniforms {
                if let Some(v) = &u.value {
                    collect_textures(v, map);
                }
                if let Some(v) = &u.location {
                    collect_textures(v, map);
                }
                if let Some(t) = &u.texture {
                    t.collect_textures(map);
                }
            }
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Shader {
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Uniform<'a> {
    #[serde(borrow, default)]
    pub name: Option<&'a str>,
    #[serde(borrow, default, rename = "type")]
    pub utype: Option<&'a str>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub location: Option<Value>,
    #[serde(borrow, default)]
    pub texture: Option<Texture<'a>>,
}

#[derive(Deserialize)]
pub(super) struct Attribute<'a> {
    #[serde(borrow, default)]
    pub name: Option<&'a str>,
    #[serde(default, rename = "bufferLength")]
    pub buffer_length: Option<Value>,
    #[serde(default, rename = "bufferBinding")]
    pub buffer_binding: Option<Value>,
}

#[derive(Deserialize)]
pub(super) struct Texture<'a> {
    #[serde(borrow, default)]
    visual: Option<&'a RawValue>,
}

impl Texture<'_> {
    /// `visual` holds data-URL previews only; a Spector object tag requires a
    /// nested `{`, so previews are skipped without a Value round-trip.
    fn collect_textures(&self, map: &mut FxHashMap<u32, RawTexData>) {
        if let Some(raw) = self.visual {
            let b = raw.get().as_bytes();
            if b.len() > 1 && memchr::memchr(b'{', &b[1..]).is_some() {
                if let Ok(v) = serde_json::from_str::<Value>(raw.get()) {
                    collect_textures(&v, map);
                }
            }
        }
    }
}
