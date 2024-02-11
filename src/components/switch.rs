use std::collections::{HashMap, LinkedList};

use bevy::prelude::*;
use bevy_rapier2d::geometry::Collider;

use crate::player::{Player, PlayerCollision};

use super::loading::Loading;

enum ScreenKind {
    Red,
    Green,
}

#[derive(Component)]
struct SwitchScreen {
    switch_name: String,
    kind: ScreenKind,
}

#[derive(Component)]
struct SwitchSensor(String);

#[derive(Component)]
pub struct Switch {
    clicked: bool,
    timer: f32,
    animation: Handle<AnimationClip>,
}

impl Switch {
    pub fn new(animations: &HashMap<String, Handle<AnimationClip>>) -> Self {
        Self {
            clicked: false,
            timer: 0.0,
            animation: animations.get("switch_pull").unwrap().clone_weak(),
        }
    }

    pub fn activated(&self) -> bool {
        self.timer >= 0.5
    }
}

pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(any_with_component::<Loading>()),
                update
                    .run_if(any_with_component::<SwitchSensor>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn init(
    mut commands: Commands,
    mut screens: Query<(Entity, &mut Visibility), (With<Loading>, With<SwitchScreen>)>,
    switches: Query<(Entity, &Name), (With<Loading>, With<Switch>)>,
    materials: Res<Assets<StandardMaterial>>,
    material_hs: Query<&Handle<StandardMaterial>>,
    parents: Query<&Children>,
    names: Query<&Name>,
    colliders: Query<&Collider>,
    mesh_hs: Query<&Handle<Mesh>>,
) {
    for (entity, switch_name) in switches.iter() {
        commands.entity(entity).remove::<Loading>();

        let mut stack = LinkedList::from([entity]);
        while let Some(current) = stack.pop_back() {
            if let Ok(name) = names.get(current).map(Name::as_str) {
                if name.contains("red") && mesh_hs.get(current).is_ok() {
                    commands
                        .entity(current)
                        .insert((
                            Loading,
                            SwitchScreen {
                                switch_name: switch_name.to_string(),
                                kind: ScreenKind::Red,
                            },
                        ))
                        .try_insert(Visibility::Hidden);
                }
                if name.contains("green") && mesh_hs.get(current).is_ok() {
                    commands
                        .entity(current)
                        .insert((
                            Loading,
                            SwitchScreen {
                                switch_name: switch_name.to_string(),
                                kind: ScreenKind::Green,
                            },
                        ))
                        .try_insert(Visibility::Hidden);
                }
                if name.contains("sensor") && colliders.get(current).is_ok() {
                    commands
                        .entity(current)
                        .insert(SwitchSensor(switch_name.to_string()));
                }
            }
            if let Ok(children) = parents.get(current) {
                stack.extend(children.into_iter());
            }
        }
    }

    for (entity, mut visibility) in screens.iter_mut() {
        commands.entity(entity).remove::<Loading>();

        *visibility = Visibility::Hidden;

        let Ok(material) = material_hs.get(entity) else {
            continue;
        };

        let mut material = materials.get(material).unwrap().clone();
        material.unlit = true;
    }
}

fn update(
    mut switches: Query<(&mut Switch, &mut AnimationPlayer, &Name)>,
    mut screens: Query<(&SwitchScreen, &mut Visibility)>,
    sensors: Query<(Entity, &SwitchSensor)>,
    player: Res<Player>,
    collisions: Query<&PlayerCollision>,
    time: Res<Time>,
) {
    for (entity, sensor) in sensors.iter() {
        let Some((mut switch, mut animation_player, switch_name)) = switches
            .iter_mut()
            .find(|(_, _, name)| name.as_str() == sensor.0)
            .map(|(switch, animation_player, name)| (switch, animation_player, name.as_str()))
        else {
            continue;
        };

        let clicked = collisions.iter().find(|c| c.other == entity).is_some() && player.is_action;

        if clicked && !switch.clicked {
            animation_player
                .play(switch.animation.clone_weak())
                .set_speed(2.0);
        }

        switch.clicked |= clicked;

        if switch.clicked {
            switch.timer += time.delta_seconds() * 2.0;
        }
        switch.timer = switch.timer.max(0.0).min(1.0);

        for (screen, mut visibility) in screens.iter_mut() {
            if screen.switch_name != switch_name {
                continue;
            }

            *visibility = Visibility::Hidden;

            match screen.kind {
                ScreenKind::Red => {
                    if !switch.activated() {
                        *visibility = Visibility::Visible
                    }
                }
                ScreenKind::Green => {
                    if switch.activated() {
                        *visibility = Visibility::Visible
                    }
                }
            }
        }
    }
}
