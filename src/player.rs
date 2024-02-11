use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_8, PI},
    time::Duration,
};

use bevy::{prelude::*, render::view::RenderLayers};
use bevy_rapier2d::prelude::*;

use crate::{
    components::loading::Loading,
    game_scene::{GameScene, GameSceneData, LoadGameScene},
    utils::reduce_to_root,
};

#[derive(Resource)]
pub struct PlayerRoot(pub Entity);

#[derive(Resource)]
pub struct LoadPlayer;

#[derive(Default, Clone, PartialEq, Debug)]
pub enum Direction {
    #[default]
    Right,
    Left,
}

pub struct ViewController {
    pub name: String,
    pub from: Vec3,
    pub to: Vec3,
    pub hide_player: bool,
}

#[derive(Resource)]
pub struct Player {
    scene_data: GameSceneData,
    pub view_controller: Option<ViewController>,
    light: Option<Entity>,
    pub oxygen: Option<Entity>,
    pub socket: Option<Entity>,
    pub is_action: bool,
    pub is_space: bool,
    is_up: bool,
    is_down: bool,
    is_left: bool,
    is_right: bool,
    pub is_mouse: bool,
    pub direction: Direction,
    pub move_vec: Vec2,
    pub push_vec: Vec2,
    swim_timer: f32,
    push_timer: f32,
    turnaround_timer: f32,
    light_timer: f32,
}

impl GameScene for Player {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            view_controller: None,
            light: None,
            oxygen: None,
            socket: None,
            is_action: false,
            is_space: false,
            is_up: false,
            is_down: false,
            is_left: false,
            is_right: false,
            is_mouse: false,
            direction: Direction::default(),
            move_vec: Vec2::ZERO,
            push_vec: Vec2::ZERO,
            swim_timer: 0.0,
            push_timer: 0.0,
            turnaround_timer: 0.0,
            light_timer: 0.0,
        }
    }
}

#[derive(Component)]
pub struct PlayerPhysics;

#[derive(Component)]
pub struct PlayerModel;

#[derive(Component)]
pub struct PlayerCollision {
    pub other: Entity,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                player_load.run_if(resource_exists::<LoadPlayer>()),
                player_ready.run_if(resource_added::<Player>()),
                (
                    process_keyboard,
                    process_movement,
                    process_view_controller,
                    process_light,
                    process_animations,
                    process_collisions,
                )
                    .run_if(resource_exists::<Player>())
                    .run_if(not(resource_added::<Player>()))
                    .run_if(not(any_with_component::<Loading>()))
                    .after(player_ready),
            ),
        );
    }
}

fn player_load(mut commands: Commands) {
    commands.remove_resource::<LoadPlayer>();

    let physics = commands
        .spawn((
            PlayerPhysics,
            Name::new("player"),
            RigidBody::Dynamic,
            TransformBundle::from_transform(Transform::from_xyz(0.0, 1.0, 0.0)),
            ExternalImpulse::default(),
            Velocity::default(),
            Collider::capsule_y(0.5, 0.5),
        ))
        .id();
    commands
        .spawn((PlayerModel, LoadGameScene::new::<Player>("diver.glb", 0)))
        .set_parent(physics);

    commands.remove_resource::<PlayerRoot>();
    commands.insert_resource(PlayerRoot(physics));
}

fn player_ready(
    mut commands: Commands,
    mut player: ResMut<Player>,
    entities: Query<(Entity, &Name)>,
    children: Query<&Parent>,
) {
    let root = player.scene_data.root;
    for (entity, name) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        match name.as_str() {
            "spine.006" => {
                commands.entity(entity).with_children(|p| {
                    let light = p.spawn((
                        SpotLightBundle {
                            transform: Transform::from_rotation(Quat::from_rotation_y(PI))
                                .with_translation(Vec3::new(1.0, 0.0, 0.0)),
                            spot_light: SpotLight {
                                intensity: 200000.0,
                                color: Color::WHITE,
                                shadows_enabled: true,
                                range: 1000.0,
                                outer_angle: 0.5 * FRAC_PI_8,
                                radius: 0.25,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        RenderLayers::from_layers(&[0, 1]),
                    ));
                    player.light = Some(light.id());
                });
            }
            "spine.007" => player.oxygen = Some(entity),
            _ => {}
        }
    }
}

fn process_keyboard(
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut player: ResMut<Player>,
) {
    player.is_action = keyboard_input.pressed(KeyCode::E);
    player.is_space = keyboard_input.pressed(KeyCode::Space);
    player.is_up = keyboard_input.pressed(KeyCode::W);
    player.is_left = keyboard_input.pressed(KeyCode::A);
    player.is_down = keyboard_input.pressed(KeyCode::S);
    player.is_right = keyboard_input.pressed(KeyCode::D);
    player.is_mouse = mouse_input.pressed(MouseButton::Left);
}

fn process_movement(
    time: Res<Time>,
    mut player: ResMut<Player>,
    mut player_physics: Query<
        (&mut ExternalImpulse, &Velocity, &Transform),
        (With<PlayerPhysics>, Without<PlayerModel>),
    >,
    mut player_model: Query<&mut Transform, (With<PlayerModel>, Without<PlayerPhysics>)>,
) {
    let lin_speed = 10.0;
    let ang_speed = 12.0;

    let lin_tmax = 0.3;
    let push_tmax = 0.1;
    let ang_tmax = 0.2;

    let (mut impulse, velocity, transform) = player_physics.single_mut();

    player.move_vec = Vec2 {
        x: (player.is_right as i32 - player.is_left as i32) as f32,
        y: (player.is_up as i32 - player.is_down as i32) as f32,
    }
    .normalize_or_zero();

    let is_moving = player.move_vec != Vec2::ZERO;
    let is_pushing = player.push_vec != Vec2::ZERO;

    player.swim_timer += time.delta_seconds();
    if !is_moving {
        player.swim_timer = 0.0;
    }

    player.push_timer += time.delta_seconds();
    if !is_pushing {
        player.push_timer = 0.0;
    }

    let lin_factor = player.swim_timer.min(lin_tmax) / lin_tmax;
    let lin_speed = transform.up().xy() * lin_speed * lin_factor;
    let push_speed = player.push_vec * player.push_timer.min(push_tmax) / push_tmax;
    impulse.impulse = lin_speed + push_speed - velocity.linvel;

    let (ang_dst, ang_factor) = if is_moving {
        (player.move_vec, player.swim_timer.min(ang_tmax) / ang_tmax)
    } else {
        (Vec2::Y, 1.0)
    };
    let ang_dir = transform.up().xy().angle_between(ang_dst);
    impulse.torque_impulse = ang_dir * ang_speed * ang_factor - velocity.angvel;

    let direction = if player.move_vec.x < 0.0 {
        Direction::Left
    } else if player.move_vec.x > 0.0 {
        Direction::Right
    } else {
        player.direction.clone()
    };

    player.turnaround_timer += time.delta_seconds();
    if direction != player.direction {
        player.direction = direction.clone();
        player.turnaround_timer = 0.0;
    }

    let rotation_directon = if player.is_space {
        FRAC_PI_2
    } else {
        match direction {
            Direction::Left => PI,
            Direction::Right => 0.0,
        }
    };

    let swaying_speed = if is_moving { 8.0 } else { 2.3 };
    let rotation_swaying = 0.5 * (swaying_speed * time.elapsed_seconds()).sin() * FRAC_PI_8;

    let swaying_speed = if is_moving { 0.0 } else { 1.7 };
    let translation_swaying = 0.1 * (swaying_speed * time.elapsed_seconds()).sin();

    let mut player_model = player_model.single_mut();
    player_model.rotation = player_model.rotation.slerp(
        Quat::from_axis_angle(Vec3::Y, rotation_directon + rotation_swaying),
        10.0 * time.delta_seconds(),
    );
    player_model.translation = player_model.translation.lerp(
        Vec3::from((0.0, translation_swaying, 0.0)),
        time.delta_seconds(),
    );
}

fn process_view_controller(
    mut visibility: Query<&mut Visibility, With<PlayerModel>>,
    player: Res<Player>,
) {
    let Ok(mut visibility) = visibility.get_single_mut() else {
        return;
    };
    if let Some(ref view) = player.view_controller {
        if view.hide_player {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    } else {
        *visibility = Visibility::Inherited;
    }
}

fn process_light(time: Res<Time>, mut player: ResMut<Player>, mut v: Query<&mut Visibility>) {
    player.light_timer = if player.is_space {
        (player.light_timer + time.delta_seconds() * 5.0).min(1.0)
    } else {
        (player.light_timer - time.delta_seconds() * 20.0).max(0.0)
    };
    let mut v = v.get_mut(player.light.unwrap()).unwrap();
    *v = if player.light_timer > 0.5 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

fn process_animations(player: Res<Player>, mut anim_player: Query<(&Name, &mut AnimationPlayer)>) {
    let idle = player.scene_data.animations.get("idle").unwrap();
    let swim = player.scene_data.animations.get("swim").unwrap();
    let (_, mut anim_player) = anim_player
        .iter_mut()
        .find(|(n, _)| n.as_str() == "player")
        .unwrap();

    let anim = if player.move_vec == Vec2::ZERO {
        idle
    } else {
        swim
    };

    if !anim_player.is_playing_clip(&anim) {
        anim_player
            .play_with_transition(anim.clone_weak(), Duration::from_millis(250))
            .repeat();
    }
}

pub fn process_collisions(
    player: Query<Entity, With<PlayerPhysics>>,
    collisions: Query<(Entity, &PlayerCollision)>,
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
) {
    let player = player.single();

    for e in collision_events.read() {
        let (other, started) = match e {
            CollisionEvent::Started(e1, e2, _) => (
                if e1 == &player {
                    Some(e2)
                } else if e2 == &player {
                    Some(e1)
                } else {
                    None
                },
                true,
            ),
            CollisionEvent::Stopped(e1, e2, _) => (
                if e1 == &player {
                    Some(e2)
                } else if e2 == &player {
                    Some(e1)
                } else {
                    None
                },
                false,
            ),
        };

        if let Some(other) = other.map(ToOwned::to_owned) {
            let collision = PlayerCollision { other };
            if started {
                commands.spawn(collision);
            } else {
                for (e, c) in collisions.iter() {
                    if c.other == collision.other {
                        commands.entity(e).despawn_recursive();
                    }
                }
            }
        }
    }
}
