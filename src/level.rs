use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    pbr::ShadowFilteringMethod,
    prelude::*,
};

use crate::{
    game_scene::{GameScene, LoadGameScene},
    player::LoadPlayer,
    GameState,
};

pub trait GameLevel {
    fn build(state: GameState, app: &mut App);
}

#[derive(Resource)]
pub struct LoadLevel {
    load: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
}

impl LoadLevel {
    pub fn new<T: Resource + GameScene>(name: &str, scene: u32) -> Self {
        Self {
            load: Some(Box::new({
                let name = name.to_string();
                move |commands| {
                    commands.spawn(LoadGameScene::new::<T>(&name, scene));
                }
            })),
        }
    }
}

#[derive(Default)]
pub struct LevelPlugin {
    levels: Vec<Box<dyn Fn(&mut App) + Send + Sync>>,
}

impl LevelPlugin {
    pub fn with_level<T: GameLevel>(mut self, state: GameState) -> Self {
        self.levels.push(Box::new({
            let state = state.clone();
            move |app| T::build(state.clone(), app)
        }));
        self
    }
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load.run_if(resource_exists::<LoadLevel>()));
        for level in &self.levels {
            level(app);
        }
    }
}

fn load(mut commands: Commands, mut level: ResMut<LoadLevel>) {
    level.load.take().unwrap()(&mut commands);
    commands.insert_resource(LoadPlayer);
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..Default::default()
            },
            transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::BlenderFilmic,
            ..default()
        },
        ShadowFilteringMethod::Castano13,
        BloomSettings {
            ..Default::default()
        },
        FogSettings {
            color: Color::hsl(180.0, 0.8, 0.1),
            directional_light_color: Color::rgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                20.0,
                Color::hsl(180.0, 0.8, 0.3),
                Color::hsl(180.0, 0.8, 0.5),
            ),
        },
    ));

    commands.remove_resource::<LoadLevel>();
}
