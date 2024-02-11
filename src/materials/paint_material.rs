use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct PaintMaterial {}

impl MaterialExtension for PaintMaterial {
    fn fragment_shader() -> ShaderRef {
        "paint_material.wgsl".into()
    }
}
