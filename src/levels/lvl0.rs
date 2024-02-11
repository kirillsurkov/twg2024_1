use anyhow::{Context, Result};
use bevy::{
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
    render::view::RenderLayers,
};

use crate::{
    components::loading::Loading, game_scene::{GameScene, GameSceneData}, handle_errors, materials::paint_material::PaintMaterial, player::{Direction, Player, PlayerCollision}, utils::reduce_to_root, GameState
};

use super::{GameLevel, LoadLevel};

#[derive(Resource)]
pub struct Level0 {
    scene_data: GameSceneData,
    lever1_clicked: bool,
    pusher1_active: bool,
}

impl GameScene for Level0 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            lever1_clicked: false,
            pusher1_active: true,
        }
    }
}

impl GameLevel for Level0 {
    fn build(state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state.clone()), setup);
        app.add_systems(OnExit(state.clone()), cleanup);
        app.add_systems(
            Update,
            (
                ready.run_if(resource_added::<Level0>()),
                (
                    process_sensors.pipe(handle_errors),
                    process_animations.pipe(handle_errors),
                )
                    .run_if(in_state(state.clone()))
                    .run_if(resource_exists::<Level0>())
                    .run_if(resource_exists::<Player>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level0>("lvl1.glb", 0));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level0>();
}

fn ready(
    mut commands: Commands,
    mut text_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, PaintMaterial>>>,
    materials: ResMut<Assets<StandardMaterial>>,
    level: Res<Level0>,
    entities: Query<(Entity, &Name, &Handle<StandardMaterial>)>,
    children: Query<&Parent>,
) {
    let root = level.scene_data.root;
    for (entity, name, mat) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        match name.as_str() {
            "text.1" => {
                let mut base = materials.get(mat).unwrap().clone();
                base.alpha_mode = AlphaMode::Blend;
                base.opaque_render_method = OpaqueRendererMethod::Forward;
                let h = text_materials.add(ExtendedMaterial {
                    base,
                    extension: PaintMaterial {},
                });
                commands.entity(entity).remove::<Handle<StandardMaterial>>();
                commands.entity(entity).insert((h, RenderLayers::layer(1)));
            }
            _ => {}
        }
    }
}

fn process_sensors(
    names: Query<&Name>,
    collisions: Query<&PlayerCollision>,
    mut level: ResMut<Level0>,
    mut player: ResMut<Player>,
) -> Result<()> {
    player.push_vec = Vec2::ZERO;

    for collision in collisions.iter() {
        match names.get(collision.other).map(|n| n.as_str()) {
            Ok("pusher1") => {
                if level.pusher1_active {
                    player.push_vec.y += 15.0
                }
            }
            Ok("lever1_sensor") => {
                if player.direction == Direction::Left && player.is_action {
                    level.pusher1_active = false;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn process_animations(
    mut level: ResMut<Level0>,
    mut anim_player: Query<(&Name, &mut AnimationPlayer)>,
) -> Result<()> {
    let clip = |scene_data: &GameSceneData, name| {
        scene_data
            .animations
            .get(name)
            .map(|c| c.clone_weak())
            .context(format!("No animation with name '{name}'"))
    };

    for (name, mut player) in anim_player.iter_mut() {
        match name.as_str() {
            "fan1" => {
                if level.pusher1_active {
                    let clip = clip(&level.scene_data, "floor_fan")?;
                    if !player.is_playing_clip(&clip) {
                        player.play(clip).repeat().set_speed(2.0);
                    }
                } else {
                    player.pause()
                }
            }
            "lever1" => {
                if !level.pusher1_active && !level.lever1_clicked {
                    level.lever1_clicked = true;
                    let clip = clip(&level.scene_data, "lever_pull")?;
                    if !player.is_playing_clip(&clip) {
                        player.play(clip);
                    }
                }
            }
            "submarine_lights" => {
                let clip = clip(&level.scene_data, "submarine_lights")?;
                if !player.is_playing_clip(&clip) {
                    player.play(clip).repeat();
                }
            }
            _ => {}
        }
    }

    Ok(())
}
