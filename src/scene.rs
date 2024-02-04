use std::collections::HashMap;

use bevy::{
    gltf::{Gltf, GltfExtras},
    pbr::TransmittedShadowReceiver,
    prelude::*,
    render::primitives::Aabb,
};
use bevy_rapier2d::prelude::*;
use serde::Deserialize;

#[derive(Component)]
pub struct Scene {
    pub animations: HashMap<String, Handle<AnimationClip>>,
}

#[derive(Component)]
pub struct SceneCleanup {
    name: String,
}

#[derive(Component)]
pub struct SceneLoad {
    gltf: Option<Handle<Gltf>>,
    root: Option<Entity>,
    name: String,
    scene: u32,
}

impl SceneLoad {
    pub fn new(name: &str, scene: u32) -> Self {
        Self {
            gltf: None,
            root: None,
            name: name.to_string(),
            scene,
        }
    }
}

#[derive(Component)]
struct SceneInit {
    name: String,
    ready: bool,
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                cleanup.run_if(any_with_component::<SceneCleanup>()),
                load.run_if(any_with_component::<SceneLoad>()),
                init.run_if(any_with_component::<SceneInit>()),
                ready.run_if(any_with_component::<SceneInit>()),
            ),
        );
    }
}

#[derive(Default, Debug, Clone, Deserialize, Component)]
struct CustomProps {
    #[serde(default)]
    ignore_physics: bool,
    #[serde(default)]
    invisible: bool,
    #[serde(default)]
    sensor: bool,
    #[serde(default)]
    diffuse_transmission: bool,
}

fn reduce_to_root<F: FnMut(T, Entity) -> T, T>(
    children: &Query<&Parent>,
    from: Entity,
    initial: T,
    mut cb: F,
) -> T {
    let mut acc = initial;
    let mut root = from;
    loop {
        acc = cb(acc, root);
        let Ok(parent) = children.get(root).map(|parent| parent.get()) else {
            break;
        };
        root = parent;
    }
    acc
}

fn cleanup(
    mut commands: Commands,
    cleanups: Query<(Entity, &SceneCleanup)>,
    entities: Query<(Entity, &Name), With<Scene>>,
) {
    println!("CLEANUP");
    for (ec, cleanup) in cleanups.iter() {
        for (es, name) in entities.iter() {
            if cleanup.name == name.as_str() {
                commands.entity(es).despawn_recursive();
            }
        }
        commands.entity(ec).despawn_recursive();
    }
}

fn load(
    mut commands: Commands,
    mut scenes: Query<(Entity, &mut SceneLoad)>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    entities: Query<(Entity, Option<&Name>)>,
    children: Query<&Parent>,
    extras: Query<&GltfExtras>,
) {
    for (root, mut scene) in scenes.iter_mut() {
        let gltf = match scene.gltf {
            Some(ref gltf) => gltf.clone_weak(),
            None => {
                let handle = asset_server.load(&scene.name);
                let handle_weak = handle.clone();
                scene.gltf = Some(handle);
                handle_weak
            }
        };

        let Some(gltf) = gltfs.get(gltf) else {
            continue;
        };

        let Some(scene_handle) = gltf.scenes.get(scene.scene as usize) else {
            continue;
        };

        let Some(root) = scene.root else {
            commands.entity(root).insert((
                Name::new(scene.name.clone()),
                SceneInit {
                    name: scene.name.clone(),
                    ready: false,
                },
                CustomProps::default(),
                Scene {
                    animations: gltf.named_animations.clone().into_iter().collect(),
                },
                SceneBundle {
                    scene: scene_handle.clone_weak(),
                    ..Default::default()
                },
            ));
            scene.root = Some(root);
            continue;
        };

        for (e, n) in entities.iter() {
            if !reduce_to_root(&children, e, false, |f, r| f || (root == r)) {
                continue;
            }
            let props = extras
                .get(e)
                .ok()
                .and_then(|extras| serde_json::from_str::<CustomProps>(&extras.value).ok())
                .unwrap_or_default();
            commands.entity(e).insert((
                SceneInit {
                    name: scene.name.clone(),
                    ready: false,
                },
                props,
            ));
        }

        commands.entity(root).remove::<SceneLoad>();
    }
}

fn init(
    mut commands: Commands,
    mut entities: Query<(Entity, &mut SceneInit)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut PointLight>,
    mats: Query<&Handle<StandardMaterial>>,
    all_props: Query<&CustomProps>,
    children: Query<&Parent>,
    aabbs: Query<(&Aabb, &GlobalTransform)>,
    names: Query<&Name>,
) {
    for (e, mut ready) in entities.iter_mut() {
        if ready.ready {
            continue;
        }

        let props = all_props.get(e).unwrap().clone();
        let props = reduce_to_root(&children, e, props, |props, r| {
            let p = all_props
                .get(r)
                .map(|props| props.clone())
                .unwrap_or_default();
            CustomProps {
                ignore_physics: p.ignore_physics || props.ignore_physics,
                invisible: p.invisible || props.invisible,
                sensor: p.sensor || props.sensor,
                diffuse_transmission: p.diffuse_transmission || props.diffuse_transmission,
            }
        });

        if let Ok(mut light) = lights.get_mut(e) {
            light.shadows_enabled = true;
            light.range = 1000.0;
            light.radius = 0.25;
        }

        if let Ok(mat) = mats.get(e) {
            let mat = materials.get_mut(mat).unwrap();
            if mat.emissive.l() != 0.0 {
                mat.fog_enabled = false;
            }
        }

        if props.invisible || props.sensor {
            commands.entity(e).insert(Visibility::Hidden);
        }

        if props.diffuse_transmission {
            commands.entity(e).insert(TransmittedShadowReceiver);
        }

        if !props.ignore_physics {
            if let Ok((aabb, transform)) = aabbs.get(e) {
                let lp1 = aabb.center - aabb.half_extents;
                let lp2 = aabb.center + aabb.half_extents;

                let gc = transform.transform_point(aabb.center.into());
                let gp1 = transform.transform_point(lp1.into());
                let gp2 = transform.transform_point(lp2.into());

                if gp1.min(gp2).z <= 0.0 && gp1.max(gp2).z >= 0.0 {
                    let (scale, rotation, _) = transform.to_scale_rotation_translation();
                    let mut new_entity = commands.spawn((
                        RigidBody::Fixed,
                        TransformBundle::from(
                            Transform::from_translation(Vec3::from((gc.xy(), 0.0)))
                                .with_rotation(rotation)
                                .with_scale(scale),
                        ),
                        Collider::cuboid(aabb.half_extents.x, aabb.half_extents.y),
                    ));
                    new_entity.set_parent(e);
                    if props.sensor {
                        new_entity.insert((Sensor, ActiveEvents::COLLISION_EVENTS));
                    }
                    if let Ok(name) = names.get(e) {
                        new_entity.insert(name.clone());
                    }
                }
            }
        }

        ready.ready = true;
    }
}

fn ready(mut commands: Commands, entities: Query<(Entity, &SceneInit)>) {
    let mut ready_map = HashMap::new();
    for (e, ready) in entities.iter() {
        *ready_map.entry(&ready.name).or_insert(true) &= ready.ready;
    }
}
