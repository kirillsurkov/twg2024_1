use anyhow::{Context, Result};
use bevy::{
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
    render::view::RenderLayers,
};

use crate::{
    camcone_material::CamConeMaterial,
    game_scene::{GameScene, GameSceneData},
    handle_errors,
    level::{GameLevel, LoadLevel},
    player::{Direction, Player, PlayerCollision},
    utils::reduce_to_root,
    GameState,
};

#[derive(Resource)]
pub struct Level1 {
    scene_data: GameSceneData,
    cam1: Option<Handle<ExtendedMaterial<StandardMaterial, CamConeMaterial>>>,
    cam1_timer: f32,
}

impl GameScene for Level1 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            cam1: None,
            cam1_timer: 0.0,
        }
    }
}

impl GameLevel for Level1 {
    fn build(state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state.clone()), setup);
        app.add_systems(OnExit(state.clone()), cleanup);
        app.add_systems(
            Update,
            (
                ready.run_if(resource_added::<Level1>()),
                (
                    process_sensors
                        .run_if(resource_exists::<Level1>())
                        .run_if(resource_exists::<Player>()),
                    process_animations
                        .pipe(handle_errors)
                        .run_if(resource_exists::<Level1>()),
                )
                    .run_if(in_state(state.clone()))
                    .after(ready),
            ),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level1>("lvl1.glb", 1));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level1>();
}

fn ready(
    mut commands: Commands,
    mut level: ResMut<Level1>,
    mut camcone_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, CamConeMaterial>>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    entities: Query<(Entity, &Name, &Handle<StandardMaterial>)>,
    children: Query<&Parent>,
) {
    let root = level.scene_data.root;
    for (entity, name, mat) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        match name.as_str() {
            "camera.1.cone" => {
                let mut base = materials.get(mat).unwrap().clone();
                base.alpha_mode = AlphaMode::Blend;
                base.double_sided = false;
                base.unlit = true;
                let h = camcone_materials.add(ExtendedMaterial {
                    base,
                    extension: CamConeMaterial::default(),
                });
                level.cam1 = Some(h.clone_weak());
                commands.entity(entity).insert(h);
                commands.entity(entity).remove::<Handle<StandardMaterial>>();

            }
            _ => {}
        }
    }
}

fn process_sensors(
    mut level: ResMut<Level1>,
    mut player: ResMut<Player>,
    mut camcone_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, CamConeMaterial>>>,
    time: Res<Time>,
    names: Query<&Name>,
    collisions: Query<&PlayerCollision>,
) {
    let mut cam1 = false;

    for c in collisions.iter() {
        match names.get(c.other).map(Name::as_str) {
            Ok("camera.1.sensor") => cam1 = true,
            _ => {}
        }
    }

    if cam1 {
        level.cam1_timer += time.delta_seconds() * 0.2;
    } else {
        level.cam1_timer -= time.delta_seconds() * 1.0;
    }
    level.cam1_timer = level.cam1_timer.max(0.0).min(1.0);

    camcone_materials
        .get_mut(level.cam1.as_ref().unwrap())
        .unwrap()
        .extension
        .amount = level.cam1_timer;
}

fn process_animations(
    mut level: ResMut<Level1>,
    mut anim_player: Query<(&Name, &mut AnimationPlayer)>,
) -> Result<()> {
    Ok(())
}
