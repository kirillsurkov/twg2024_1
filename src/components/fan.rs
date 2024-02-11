use std::{
    collections::{HashMap, LinkedList},
    f32::consts::PI,
    time::Duration,
};

use bevy::prelude::*;
use bevy_rapier2d::geometry::{Collider, Sensor};

use crate::player::{Player, PlayerCollision};

use super::loading::Loading;

#[derive(Component)]
pub struct Fan {
    pub spinning: bool,
    factor: f32,
    pusher: Option<Entity>,
    model: Option<Entity>,
}

impl Fan {
    pub fn new() -> Self {
        Self {
            spinning: true,
            factor: 1.0,
            pusher: None,
            model: None,
        }
    }
}

pub struct FanPlugin;

impl Plugin for FanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(any_with_component::<Loading>()),
                update
                    .run_if(any_with_component::<Fan>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn init(
    mut commands: Commands,
    mut fans: Query<(Entity, &mut Fan), With<Loading>>,
    parents: Query<&Children>,
    names: Query<&Name>,
    colliders: Query<&Collider>,
) {
    for (entity, mut fan) in fans.iter_mut() {
        commands.entity(entity).remove::<Loading>();

        let mut pusher = None;
        let mut model = None;

        let mut stack = LinkedList::from([entity]);
        while let Some(current) = stack.pop_back() {
            if let Ok(name) = names.get(current).map(Name::as_str) {
                if name.contains("pusher") && colliders.get(current).is_ok() {
                    pusher = Some(current);
                }
                if name.contains("fan.") {
                    model = Some(current);
                }
            }
            if let Ok(children) = parents.get(current) {
                stack.extend(children.into_iter());
            }
        }

        fan.pusher = pusher;
        fan.model = model;
    }
}

fn update(
    mut commands: Commands,
    mut player: ResMut<Player>,
    mut fans: Query<(&mut Fan, &GlobalTransform)>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
    collisions: Query<&PlayerCollision>,
) {
    player.push_vec = Vec2::ZERO;
    for (mut fan, transform_g) in fans.iter_mut() {
        transforms
            .get_mut(fan.model.unwrap())
            .unwrap()
            .rotate_y(fan.factor * 10.0 * PI * time.delta_seconds());
        if fan.spinning {
            let pusher = fan.pusher.unwrap();
            if collisions.iter().find(|c| c.other == pusher).is_some() {
                player.push_vec += transform_g.up().xy().normalize_or_zero() * 15.0;
            }
        } else {
            fan.factor -= time.delta_seconds();
            fan.factor = fan.factor.max(0.0).min(1.0);
        }
    }
}
