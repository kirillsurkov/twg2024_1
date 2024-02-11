use anyhow::Result;
use bevy::{
    log::{self, LogPlugin},
    pbr::ExtendedMaterial,
    prelude::*,
};
use bevy_hanabi::HanabiPlugin;
use bevy_inspector_egui::{quick::WorldInspectorPlugin, DefaultInspectorConfigPlugin};
use bevy_mod_raycast::DefaultRaycastingPlugin;
use bevy_rapier2d::prelude::*;
use camera::CameraPlugin;
use components::{
    code::CodePlugin, fan::FanPlugin, gate::GatePlugin, security_camera::SecurityCameraPlugin,
    socket::SocketPlugin, switch::SwitchPlugin,
};
use game_scene::GameScenePlugin;
use levels::{lvl0::Level0, lvl1::Level1, lvl2::Level2, lvl3::Level3, lvl4::Level4, LevelPlugin};
use materials::{beam_material::BeamMaterial, paint_material::PaintMaterial};
use mips::{generate_mipmaps, MipmapGeneratorPlugin};
use player::PlayerPlugin;

mod mips;

mod camera;
mod components;
mod game_scene;
mod levels;
mod materials;
mod player;
mod utils;

mod level_generator;

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, States)]
enum GameState {
    Restart,
    Level0,
    Level1,
    #[default]
    Level2,
    Level3,
    Level4,
}

#[derive(Resource)]
pub struct Restart(GameState);

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
            DefaultPlugins
                .set(bevy_mod_raycast::low_latency_window_plugin())
                .set(LogPlugin {
                    level: log::Level::ERROR,
                    ..Default::default()
                }),
            DefaultRaycastingPlugin,
            MipmapGeneratorPlugin,
            //HanabiPlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            //RapierDebugRenderPlugin::default(),
            //WorldInspectorPlugin::new(),
        ))
        .add_systems(Update, generate_mipmaps::<StandardMaterial>)
        .add_plugins((
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, PaintMaterial>>::default(),
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, BeamMaterial>>::default(),
            SecurityCameraPlugin,
            SwitchPlugin,
            GatePlugin,
            CodePlugin,
            SocketPlugin,
            FanPlugin,
            GameScenePlugin,
            CameraPlugin,
            PlayerPlugin,
            LevelPlugin::default()
                .with_level::<Level0>(GameState::Level0)
                .with_level::<Level1>(GameState::Level1)
                .with_level::<Level2>(GameState::Level2)
                .with_level::<Level3>(GameState::Level3)
                .with_level::<Level4>(GameState::Level4),
        ))
        .add_state::<GameState>()
        .run();
}
