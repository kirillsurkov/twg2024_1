use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct BeamMaterial {
    #[uniform(100)]
    pub color: Vec3,
    #[uniform(100)]
    pub visibility: f32,
}

impl MaterialExtension for BeamMaterial {
    fn fragment_shader() -> ShaderRef {
        "beam_material.wgsl".into()
    }
}
