use anyhow::Result;
use bevy::{pbr::ExtendedMaterial, prelude::*};
use bevy_hanabi::HanabiPlugin;
use bevy_inspector_egui::{quick::WorldInspectorPlugin, DefaultInspectorConfigPlugin};
use bevy_rapier2d::prelude::*;
use camcone_material::CamConeMaterial;
use camera::CameraPlugin;
use game_scene::GameScenePlugin;
use level::LevelPlugin;
use lvl0::Level0;
use lvl1::Level1;
use mips::{generate_mipmaps, MipmapGeneratorPlugin};
use paint_material::PaintMaterial;
use player::PlayerPlugin;

mod mips;

mod camera;
mod game_scene;
mod level;
mod player;
mod utils;

mod camcone_material;
mod paint_material;

mod level_generator;

mod lvl0;
mod lvl1;

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, States)]
enum GameState {
    Level0,
    #[default]
    Level1,
}

pub(crate) fn handle_errors(In(result): In<Result<()>>) {
    if let Err(e) = result {
        eprintln!("System early returned: {}", e);
    }
}

fn main() {
    App::new()
        .insert_resource(RapierConfiguration {
            gravity: Vec2::ZERO,
            ..Default::default()
        })
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.0,
        })
        .add_plugins((
            DefaultPlugins,
            MipmapGeneratorPlugin,
            //HanabiPlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            //RapierDebugRenderPlugin::default(),
            //WorldInspectorPlugin::new(),
        ))
        .add_systems(Update, generate_mipmaps::<StandardMaterial>)
        .add_plugins((
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, PaintMaterial>>::default(),
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, CamConeMaterial>>::default(),
            GameScenePlugin,
            CameraPlugin,
            PlayerPlugin,
            LevelPlugin::default()
                .with_level::<Level0>(GameState::Level0)
                .with_level::<Level1>(GameState::Level1),
        ))
        .add_state::<GameState>()
        .run();
}
