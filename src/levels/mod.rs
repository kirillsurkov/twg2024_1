use std::{collections::HashMap, f32::consts::FRAC_PI_3};

use bevy::{
    core_pipeline::{
        bloom::BloomSettings, clear_color::ClearColorConfig, core_3d::Camera3dDepthLoadOp,
        tonemapping::Tonemapping,
    },
    pbr::ShadowFilteringMethod,
    prelude::*,
    render::view::RenderLayers,
};

use crate::{
    game_scene::{GameScene, LoadGameScene},
    player::{LoadPlayer, Player, PlayerRoot},
    GameState, Restart,
};

pub mod lvl0;
pub mod lvl1;
pub mod lvl2;
pub mod lvl3;
pub mod lvl4;

pub trait GameLevel {
    fn build(state: GameState, app: &mut App);
}

#[derive(Resource)]
pub struct LevelRoot(Entity);

#[derive(Resource)]
pub struct LoadLevel {
    load: Option<Box<dyn FnOnce(&mut Commands) -> Entity + Send + Sync>>,
}

impl LoadLevel {
    pub fn new<T: Resource + GameScene>(name: &str, scene: u32) -> Self {
        Self {
            load: Some(Box::new({
                let name = name.to_string();
                move |commands| {
                    commands.remove_resource::<LoadLevel>();
                    commands.insert_resource(LoadPlayer);

                    let parent = commands.spawn(LoadGameScene::new::<T>(&name, scene)).id();

                    spawn_camera(commands, 0, parent);
                    spawn_camera(commands, 1, parent);

                    parent
                }
            })),
        }
    }
}

#[derive(Default)]
pub struct LevelPlugin {
    levels: HashMap<GameState, Box<dyn Fn(&mut App) + Send + Sync>>,
}

impl LevelPlugin {
    pub fn with_level<T: GameLevel>(mut self, state: GameState) -> Self {
        self.levels.insert(
            state.clone(),
            Box::new({
                let state = state.clone();
                move |app| T::build(state.clone(), app)
            }),
        );
        self
    }
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load.run_if(resource_exists::<LoadLevel>()));
        for (state, level) in &self.levels {
            app.add_systems(OnExit(state.clone()), cleanup);
            level(app);
        }
        app.add_systems(OnEnter(GameState::Restart), restart);
    }
}

fn restart(
    mut commands: Commands,
    mut game_state: ResMut<NextState<GameState>>,
    restart: Res<Restart>,
) {
    commands.remove_resource::<Restart>();
    game_state.set(restart.0.clone());
}

fn cleanup(mut commands: Commands, level_root: Res<LevelRoot>, player_root: Res<PlayerRoot>) {
    commands.remove_resource::<Player>();
    commands.entity(player_root.0).despawn_recursive();
    commands.entity(level_root.0).despawn_recursive();
}

fn spawn_camera(commands: &mut Commands, order: u8, parent: Entity) {
    let clear_color = if order == 0 {
        ClearColorConfig::Custom(Color::BLACK)
    } else {
        ClearColorConfig::None
    };
    let depth_load_op = if order == 0 {
        Camera3dDepthLoadOp::Clear(0.0)
    } else {
        Camera3dDepthLoadOp::Load
    };
    commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    order: order as isize,
                    ..Default::default()
                },
                camera_3d: Camera3d {
                    clear_color,
                    depth_load_op,
                    ..Default::default()
                },
                transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
                tonemapping: Tonemapping::BlenderFilmic,
                projection: Projection::Perspective(PerspectiveProjection {
                    fov: FRAC_PI_3,
                    near: 0.01,
                    far: 100.0,
                    ..Default::default()
                }),
                ..Default::default()
            },
            RenderLayers::layer(order),
            //ShadowFilteringMethod::Castano13,
            BloomSettings::default(),
            FogSettings {
                color: Color::hsl(180.0, 0.8, 0.1),
                falloff: FogFalloff::from_visibility_colors(
                    15.0,
                    Color::hsla(180.0, 0.8, 0.3, 1.0),
                    Color::hsla(180.0, 0.8, 0.5, 1.0),
                ),
                ..Default::default()
            },
        ))
        .set_parent(parent);
}

fn load(mut commands: Commands, mut level: ResMut<LoadLevel>) {
    let root = level.load.take().unwrap()(&mut commands);
    commands.remove_resource::<LevelRoot>();
    commands.insert_resource(LevelRoot(root));
}
