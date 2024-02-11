use std::collections::LinkedList;

use bevy::{pbr::NotShadowReceiver, prelude::*};
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings, RaycastVisibility},
    primitives::Ray3d,
};
use bevy_rapier2d::geometry::{Collider, Sensor};

use crate::{
    player::{Player, PlayerCollision, PlayerPhysics},
    utils::reduce_to_root,
};

use super::{loading::Loading, security_camera::SecurityCamera};

#[derive(Debug)]
enum State {
    CanCarryFrom,
    CanCarryTo,
    Carrying,
    ConnectedTo(Entity),
    ConnectedFrom,
}

#[derive(Component, Debug)]
pub struct Socket {
    sensor: Option<Entity>,
    wire: Option<Entity>,
    state: State,
    is_action_last: bool,
    break_timer: f32,
    camera: Option<Entity>,
}

impl Socket {
    pub fn new(start: bool) -> Self {
        Self {
            sensor: None,
            wire: None,
            state: if start {
                State::CanCarryFrom
            } else {
                State::CanCarryTo
            },
            is_action_last: false,
            break_timer: 0.0,
            camera: None,
        }
    }
}

pub struct SocketPlugin;

impl Plugin for SocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(any_with_component::<Loading>()),
                (update, wire)
                    .run_if(any_with_component::<Socket>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn init(
    mut commands: Commands,
    mut sockets: Query<(Entity, &mut Socket), With<Loading>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    parents: Query<&Children>,
    names: Query<&Name>,
    colliders: Query<&Collider>,
) {
    for (entity, mut socket) in sockets.iter_mut() {
        commands.entity(entity).remove::<Loading>();

        let mut stack = LinkedList::from([entity]);
        while let Some(current) = stack.pop_back() {
            if let Ok(name) = names.get(current).map(Name::as_str) {
                if name.contains("sensor") && colliders.get(current).is_ok() {
                    socket.sensor = Some(current);
                }
            }
            if let Ok(children) = parents.get(current) {
                stack.extend(children.into_iter());
            }
        }

        commands.entity(socket.sensor.unwrap()).with_children(|p| {
            socket.wire = Some(
                p.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Cylinder {
                            ..Default::default()
                        })),
                        material: materials.add(StandardMaterial::default()),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    NotShadowReceiver,
                ))
                .id(),
            );
        });
    }
}

fn update(
    mut player: ResMut<Player>,
    mut sockets: Query<(Entity, &mut Socket)>,
    mut cams: Query<&mut SecurityCamera>,
    mut raycast: Raycast,
    time: Res<Time>,
    transforms_g: Query<&GlobalTransform>,
    collisions: Query<&PlayerCollision>,
    parents: Query<&Children>,
    children: Query<&Parent>,
    wire_filter: Query<(), (With<Collider>, Without<Sensor>, Without<PlayerPhysics>)>,
) {
    let mut inside = None;
    let mut carrying = None;
    let mut is_acted = false;

    for (entity, mut socket) in sockets.iter_mut() {
        let sensor = socket.sensor.unwrap();
        let is_inside = collisions.iter().find(|c| c.other == sensor).is_some();

        let acted = !socket.is_action_last && player.is_action;
        socket.is_action_last = player.is_action;

        match socket.state {
            State::CanCarryFrom => {
                if is_inside && acted {
                    socket.state = State::Carrying;
                    player.socket = Some(entity);
                }
            }
            State::CanCarryTo => {
                if is_inside {
                    inside = Some(entity);
                }
            }
            State::Carrying => {
                carrying = Some(entity);
                is_acted = acted;
            }
            _ => {}
        }
    }

    let Some(carrying) = carrying else {
        return;
    };

    let (breaking, camera) = {
        let from = transforms_g
            .get(player.oxygen.unwrap())
            .unwrap()
            .transform_point(Vec3::ZERO);
        let to = transforms_g
            .get(sockets.get(carrying).unwrap().1.sensor.unwrap())
            .unwrap()
            .transform_point(Vec3::ZERO);

        let mut distance = 0.0;

        let breaking = if let [(isec, data)] = raycast.cast_ray(
            Ray3d::new(from, to - from),
            &RaycastSettings {
                filter: &|e| {
                    parents
                        .get(e)
                        .map(|children| children.iter().all(|e| wire_filter.get(*e).is_ok()))
                        .unwrap_or_default()
                },
                visibility: RaycastVisibility::Ignore,
                ..Default::default()
            },
        ) {
            distance = data.distance();
            !reduce_to_root(&children, *isec, false, |f, p| f || (p == carrying))
        } else {
            false
        };

        let camera = if let [(isec, data)] = raycast.cast_ray(
            Ray3d::new(from, to - from),
            &RaycastSettings {
                filter: &|e| reduce_to_root(&children, e, false, |f, p| f || cams.get(p).is_ok()),
                visibility: RaycastVisibility::Ignore,
                ..Default::default()
            },
        ) {
            if data.distance() < distance {
                Some(isec)
            } else {
                None
            }
        } else {
            None
        };

        (breaking, camera)
    };

    {
        let (_, mut socket) = sockets.get_mut(carrying).unwrap();
        if let Some(camera) = camera {
            let camera = reduce_to_root(&children, *camera, *camera, |camera, parent| {
                if cams.contains(parent) {
                    parent
                } else {
                    camera
                }
            });
            socket.camera = Some(camera);
            cams.get_mut(camera).unwrap().wire = true;
        } else if let Some(camera) = socket.camera {
            socket.camera = None;
            cams.get_mut(camera).unwrap().wire = false;
        }
    }

    {
        let (_, mut socket) = sockets.get_mut(carrying).unwrap();
        if breaking {
            socket.break_timer += time.delta_seconds() * 2.0;
        } else {
            socket.break_timer -= time.delta_seconds() * 5.0;
        }
        if socket.break_timer >= 1.0 {
            socket.state = State::CanCarryFrom;
            socket.break_timer = 0.0;
            return;
        }
        socket.break_timer = socket.break_timer.max(0.0).min(1.0);
    }

    match inside {
        Some(inside) => {
            let [(_, mut carrying), (to, mut inside)] =
                sockets.get_many_mut([carrying, inside]).unwrap();

            if is_acted {
                carrying.state = State::ConnectedTo(to);
                inside.state = State::ConnectedFrom;
            }
        }
        None => {
            if is_acted {
                let (_, mut carrying) = sockets.get_mut(carrying).unwrap();
                carrying.state = State::CanCarryFrom;
            }
        }
    }
}

fn wire(
    mut visibility: Query<&mut Visibility>,
    mut transforms: Query<&mut Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    material_hs: Query<&Handle<StandardMaterial>>,
    player: Res<Player>,
    sockets: Query<&mut Socket>,
    transforms_g: Query<&GlobalTransform>,
) {
    for socket in sockets.iter() {
        let e1 = socket.sensor.unwrap();
        let wire = socket.wire.unwrap();
        let mut visibility = visibility.get_mut(wire).unwrap();

        let e2 = match socket.state {
            State::ConnectedTo(e2) => sockets.get(e2).unwrap().sensor.unwrap(),
            State::Carrying => player.oxygen.unwrap(),
            _ => {
                *visibility = Visibility::Hidden;
                continue;
            }
        };

        let gt1 = transforms_g.get(e1).unwrap();
        let gt2 = transforms_g.get(e2).unwrap();
        let gp1 = gt1.transform_point(Vec3::ZERO);
        let gp2 = gt2.transform_point(Vec3::ZERO);
        let dist = gp1.distance(gp2);
        let middle = (gp1 + gp2) / 2.0;
        let angle = Vec2::Y.angle_between((gp2 - gp1).xy());

        *transforms.get_mut(wire).unwrap() = GlobalTransform::from(
            Transform::from_translation(middle)
                .with_rotation(Quat::from_rotation_z(angle))
                .with_scale(Vec3::new(0.05, dist, 0.05)),
        )
        .reparented_to(transforms_g.get(e1).unwrap());

        let color_1 = Vec3::new(0.25, 0.25, 1.0);
        let color_2 = Vec3::new(1.0, 0.0, 0.0);
        let color = color_1.lerp(color_2, socket.break_timer);
        let material = materials.get_mut(material_hs.get(wire).unwrap()).unwrap();
        material.base_color = Color::rgb_linear(1.0, 1.0, 1.0);
        material.emissive = Color::rgb_linear(color.x, color.y, color.z) * 20.0;

        *visibility = Visibility::Visible;
    }
}
