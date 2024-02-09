use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct CamConeMaterial {
    #[uniform(100)]
    pub amount: f32,
}

impl MaterialExtension for CamConeMaterial {
    fn fragment_shader() -> ShaderRef {
        "camcone_material.wgsl".into()
    }
}
