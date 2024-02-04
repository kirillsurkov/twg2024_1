use anyhow::Result;
use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use camera::CameraPlugin;
use game_scene::GameScenePlugin;
use level::LevelPlugin;
use lvl1::Level1;
use mips::{generate_mipmaps, MipmapGeneratorPlugin};
use player::PlayerPlugin;

mod mips;

mod camera;
mod game_scene;
mod level;
mod player;

mod lvl1;

trait Lerp {
    fn lerp(&self, other: Self, scalar: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, other: Self, scalar: f32) -> Self {
        self + (other - self) * scalar
    }
}

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, States)]
enum GameState {
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
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins,
            MipmapGeneratorPlugin,
            HanabiPlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
            WorldInspectorPlugin::new(),
        ))
        .add_systems(Update, generate_mipmaps::<StandardMaterial>)
        .add_plugins((
            GameScenePlugin,
            CameraPlugin,
            PlayerPlugin,
            LevelPlugin::default().with_level::<Level1>(GameState::Level1),
        ))
        .add_state::<GameState>()
        .run();
}