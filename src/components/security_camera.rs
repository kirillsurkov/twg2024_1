use std::{collections::LinkedList, f32::consts::FRAC_PI_2};

use bevy::{pbr::ExtendedMaterial, prelude::*};
use bevy_rapier2d::geometry::Collider;

use crate::{materials::beam_material::BeamMaterial, player::PlayerCollision};

use super::loading::Loading;

#[derive(Component)]
struct CamCone {
    camera_name: String,
    material: Option<Handle<ExtendedMaterial<StandardMaterial, BeamMaterial>>>,
    light: Option<Entity>,
}

#[derive(Component)]
struct CamSensor {
    camera_name: String,
    timer: f32,
}

#[derive(Component)]
pub struct SecurityCamera {
    pub active: bool,
    triggered: bool,
    pub wire: bool,
}

impl SecurityCamera {
    pub fn new() -> Self {
        Self {
            active: true,
            triggered: false,
            wire: false,
        }
    }
}

pub struct SecurityCameraPlugin;

impl Plugin for SecurityCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(any_with_component::<Loading>()),
                update
                    .run_if(any_with_component::<CamSensor>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn init(
    mut commands: Commands,
    mut camcone_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, BeamMaterial>>>,
    mut cones: Query<(Entity, &mut CamCone), With<Loading>>,
    cameras: Query<(Entity, &Name), (With<Loading>, With<SecurityCamera>)>,
    materials: Res<Assets<StandardMaterial>>,
    material_hs: Query<&Handle<StandardMaterial>>,
    parents: Query<&Children>,
    names: Query<&Name>,
    colliders: Query<&Collider>,
    mesh_hs: Query<&Handle<Mesh>>,
) {
    for (entity, camera_name) in cameras.iter() {
        commands.entity(entity).remove::<Loading>();

        let mut stack = LinkedList::from([entity]);
        while let Some(current) = stack.pop_back() {
            if let Ok(name) = names.get(current).map(Name::as_str) {
                if name.contains("cone") && mesh_hs.get(current).is_ok() {
                    commands.entity(current).insert((
                        Loading,
                        CamCone {
                            camera_name: camera_name.to_string(),
                            material: None,
                            light: None,
                        },
                    ));
                }
                if name.contains("sensor") && colliders.get(current).is_ok() {
                    commands.entity(current).insert(CamSensor {
                        camera_name: camera_name.to_string(),
                        timer: 0.0,
                    });
                }
            }
            if let Ok(children) = parents.get(current) {
                stack.extend(children.into_iter());
            }
        }
    }

    for (entity, mut cone) in cones.iter_mut() {
        commands.entity(entity).remove::<Loading>();

        let Ok(material) = material_hs.get(entity) else {
            continue;
        };
        let mut base = materials.get(material).unwrap().clone();
        base.alpha_mode = AlphaMode::Blend;
        base.unlit = true;
        let h = camcone_materials.add(ExtendedMaterial {
            base,
            extension: BeamMaterial::default(),
        });

        cone.material = Some(h.clone_weak());
        commands.entity(entity).insert(h);
        commands.entity(entity).remove::<Handle<StandardMaterial>>();
        commands.entity(entity).with_children(|p| {
            cone.light = Some(
                p.spawn(SpotLightBundle {
                    spot_light: SpotLight {
                        range: 1000.0,
                        radius: 0.25,
                        intensity: 200000.0,
                        shadows_enabled: true,
                        inner_angle: 0.0,
                        outer_angle: 30.0f32.to_radians(),
                        ..Default::default()
                    },
                    transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                    ..Default::default()
                })
                .id(),
            );
        });
    }
}

fn update(
    mut cameras: Query<(&mut SecurityCamera, &Name)>,
    mut sensors: Query<(Entity, &mut CamSensor)>,
    mut spotlights: Query<&mut SpotLight>,
    mut camcone_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, BeamMaterial>>>,
    cones: Query<&CamCone>,
    collisions: Query<&PlayerCollision>,
    time: Res<Time>,
) {
    let color_1 = Vec3::new(0.0, 1.0, 1.0);
    let color_2 = Vec3::new(1.0, 1.0, 0.0);
    let color_3 = Vec3::new(1.0, 0.0, 0.0);

    for (entity, mut sensor) in sensors.iter_mut() {
        let Some((mut camera, camera_name)) = cameras
            .iter_mut()
            .find(|(_, name)| name.as_str() == sensor.camera_name)
            .map(|(camera, name)| (camera, name.as_str()))
        else {
            continue;
        };

        if camera.triggered {
            continue;
        }

        let interacting = collisions.iter().find(|c| c.other == entity).is_some() || camera.wire;
        if interacting && camera.active {
            sensor.timer += time.delta_seconds() * 0.2;
        } else {
            sensor.timer -= time.delta_seconds() * 1.0;
        }
        sensor.timer = sensor.timer.max(0.0);
        if sensor.timer > 1.0 {
            camera.triggered = true;
            sensor.timer = 1.0;
        }

        for cone in cones.iter() {
            if cone.camera_name != camera_name {
                continue;
            }
            let Some(ref cone_material) = cone.material else {
                continue;
            };
            let Some(cone_light) = cone.light else {
                continue;
            };

            let color = color_1
                .lerp(color_2, (sensor.timer * 3.0).clamp(0.0, 1.0))
                .lerp(color_3, (sensor.timer * 3.0 - 1.0).clamp(0.0, 2.0) / 2.0);

            let material = camcone_materials.get_mut(cone_material).unwrap();
            material.extension.color = color;
            if camera.active {
                material.extension.visibility += time.delta_seconds() * 2.0;
            } else {
                material.extension.visibility -= time.delta_seconds() * 2.0;
            }
            material.extension.visibility = material.extension.visibility.max(0.0).min(1.0);

            let mut light = spotlights.get_mut(cone_light).unwrap();
            light.color = Color::rgb(color.x, color.y, color.z);
            light.intensity = material.extension.visibility * (40000.0 + sensor.timer * 160000.0);
        }
    }
}
