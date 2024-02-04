use anyhow::{Context, Result};
use bevy::prelude::*;

use crate::{
    game_scene::{GameScene, GameSceneData},
    handle_errors,
    level::{GameLevel, LoadLevel},
    player::{Direction, Player, PlayerCollision},
    GameState,
};

#[derive(Resource)]
pub struct Level1 {
    scene_data: GameSceneData,
    lever1_clicked: bool,
    pusher1_active: bool,
}

impl GameScene for Level1 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            lever1_clicked: false,
            pusher1_active: true,
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
                process_sensors
                    .pipe(handle_errors)
                    .run_if(resource_exists::<Level1>())
                    .run_if(resource_exists::<Player>()),
                process_animations
                    .pipe(handle_errors)
                    .run_if(resource_exists::<Level1>()),
            )
                .run_if(in_state(state.clone())),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level1>("lvl1.glb", 0));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level1>();
}

fn process_sensors(
    names: Query<&Name>,
    collisions: Query<&PlayerCollision>,
    mut level: ResMut<Level1>,
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
    mut level: ResMut<Level1>,
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
                        println!("SPINNING {clip:?}");
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
