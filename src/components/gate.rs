use std::{
    collections::{HashMap, LinkedList},
    time::Duration,
};

use bevy::prelude::*;
use bevy_rapier2d::geometry::{Collider, Sensor};

use super::loading::Loading;

#[derive(Component)]
struct GatePhysics(String);

#[derive(Component)]
pub struct Gate {
    is_open: bool,
    start_animation: bool,
    animation: Handle<AnimationClip>,
}

impl Gate {
    pub fn new(animations: &HashMap<String, Handle<AnimationClip>>) -> Self {
        Self {
            is_open: false,
            start_animation: false,
            animation: animations.get("gate_open").unwrap().clone_weak(),
        }
    }

    pub fn opened(&self) -> bool {
        self.is_open && self.start_animation == false
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.start_animation = true;
    }
}

pub struct GatePlugin;

impl Plugin for GatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(any_with_component::<Loading>()),
                update
                    .run_if(any_with_component::<GatePhysics>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn init(
    mut commands: Commands,
    gates: Query<(Entity, &Name), (With<Loading>, With<Gate>)>,
    parents: Query<&Children>,
    names: Query<&Name>,
    colliders: Query<&Collider>,
) {
    for (entity, gate_name) in gates.iter() {
        commands.entity(entity).remove::<Loading>();

        let mut stack = LinkedList::from([entity]);
        while let Some(current) = stack.pop_back() {
            if let Ok(name) = names.get(current).map(Name::as_str) {
                if name.contains("physics") && colliders.get(current).is_ok() {
                    commands
                        .entity(current)
                        .insert(GatePhysics(gate_name.to_string()));
                }
            }
            if let Ok(children) = parents.get(current) {
                stack.extend(children.into_iter());
            }
        }
    }
}

fn update(
    mut commands: Commands,
    mut gates: Query<(&mut Gate, &mut AnimationPlayer, &Name)>,
    physics: Query<(Entity, &GatePhysics)>,
) {
    for (mut gate, mut animation_player, gate_name) in gates.iter_mut() {
        let (entity, _) = physics
            .iter()
            .find(|(_, physics)| physics.0 == gate_name.as_str())
            .unwrap();

        if gate.start_animation {
            gate.start_animation = false;
            animation_player
                .play_with_transition(gate.animation.clone_weak(), Duration::from_millis(250))
                .set_speed(if gate.is_open { 1.0 } else { -1.0 });
            if !gate.is_open {
                commands.entity(entity).remove::<Sensor>();
            }
        }

        if animation_player.is_finished() && gate.is_open {
            commands.entity(entity).try_insert(Sensor);
        }
    }
}
