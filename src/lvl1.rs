use anyhow::{Context, Result};
use bevy::prelude::*;

use crate::{
    handle_errors,
    level::{GameLevel, LevelAnimations, LevelLoad, LevelTag},
    player::{Direction, Player, PlayerCollision},
    GameState,
};

#[derive(Resource)]
pub struct Level1Data {
    lever1_clicked: bool,
    pusher1_active: bool,
}

impl Level1Data {
    pub fn new() -> Self {
        Self {
            lever1_clicked: false,
            pusher1_active: true,
        }
    }
}

pub struct Level1;

impl GameLevel for Level1 {
    fn on_enter(&self, state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state), setup);
    }

    fn on_exit(&self, state: GameState, app: &mut App) {
        app.add_systems(OnExit(state), cleanup);
    }

    fn update(&self, state: GameState, app: &mut App) {
        app.add_systems(
            Update,
            (
                process_sensors.pipe(handle_errors),
                process_animations
                    .pipe(handle_errors)
                    .run_if(resource_exists::<LevelAnimations>()),
            )
                .run_if(in_state(state)),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LevelLoad::new("lvl1.glb", 0));
    commands.insert_resource(Level1Data::new());
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level1Data>();
}

fn process_sensors(
    names: Query<&Name>,
    collisions: Query<&PlayerCollision>,
    mut level: ResMut<Level1Data>,
    mut player: Query<&mut Player>,
) -> Result<()> {
    let mut player = player.get_single_mut()?;

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
    mut level: ResMut<Level1Data>,
    mut anim_player: Query<(&Name, &mut AnimationPlayer), With<LevelTag>>,
    anims: Res<LevelAnimations>,
) -> Result<()> {
    let clip = |name| {
        anims
            .named
            .get(name)
            .map(|c| c.clone_weak())
            .context(format!("No animation with name '{name}'"))
    };

    for (name, mut player) in anim_player.iter_mut() {
        match name.as_str() {
            "fan1" | "fan2" => {
                if level.pusher1_active {
                    let clip = clip("floor_fan")?;
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
                    let clip = clip("lever_pull")?;
                    if !player.is_playing_clip(&clip) {
                        player.play(clip);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
