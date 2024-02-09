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

fn spawn_camera(commands: &mut Commands, order: u8) {
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
    commands.spawn((
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
                near: 0.01,
                far: 100.0,
                ..Default::default()
            }),
            ..Default::default()
        },
        RenderLayers::layer(order),
        ShadowFilteringMethod::Castano13,
        BloomSettings::default(),
        FogSettings {
            color: Color::hsl(180.0, 0.8, 0.1),
            falloff: FogFalloff::from_visibility_colors(
                20.0,
                Color::hsl(180.0, 0.8, 0.3),
                Color::hsl(180.0, 0.8, 0.5),
            ),
            ..Default::default()
        },
    ));
}

fn load(mut commands: Commands, mut level: ResMut<LoadLevel>) {
    level.load.take().unwrap()(&mut commands);
    commands.insert_resource(LoadPlayer);

    spawn_camera(&mut commands, 0);
    spawn_camera(&mut commands, 1);
    spawn_camera(&mut commands, 2);

    commands.remove_resource::<LoadLevel>();
}
