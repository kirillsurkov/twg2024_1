use std::collections::HashMap;

use bevy::{
    gltf::{Gltf, GltfExtras},
    pbr::TransmittedShadowReceiver,
    prelude::*,
    render::primitives::Aabb,
};
use bevy_rapier2d::geometry::{ActiveEvents, Collider, Sensor};
use serde::Deserialize;

pub struct GameSceneData {
    pub animations: HashMap<String, Handle<AnimationClip>>,
}

pub trait GameScene {
    fn from_scene_data(data: GameSceneData) -> Self;
}

pub struct GameScenePlugin;

impl Plugin for GameScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load);
    }
}

#[derive(Component)]
pub struct LoadGameScene {
    name: String,
    scene: u32,
    on_ready: Option<Box<dyn FnOnce(&mut Commands, GameSceneData) + Send + Sync>>,
    gltf: Option<Handle<Gltf>>,
    root: Option<Entity>,
}

impl LoadGameScene {
    pub fn new<T: Resource + GameScene>(name: &str, scene: u32) -> Self {
        Self {
            name: name.to_string(),
            scene,
            on_ready: Some(Box::new(move |commands, scene_data| {
                commands.insert_resource(T::from_scene_data(scene_data))
            })),
            gltf: None,
            root: None,
        }
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

fn load(
    mut commands: Commands,
    mut scenes: Query<(Entity, &mut LoadGameScene)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut PointLight>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    entities: Query<Entity>,
    children: Query<&Parent>,
    extras: Query<&GltfExtras>,
    mats: Query<&Handle<StandardMaterial>>,
    aabbs: Query<(&Aabb, &GlobalTransform)>,
    names: Query<&Name>,
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
                CustomProps::default(),
                SceneBundle {
                    scene: scene_handle.clone_weak(),
                    ..Default::default()
                },
            ));
            scene.root = Some(root);
            continue;
        };

        let mut all_props = HashMap::<Entity, CustomProps>::new();
        let mut prop = |entity| {
            all_props
                .entry(entity)
                .or_insert(
                    extras
                        .get(entity)
                        .ok()
                        .and_then(|extras| serde_json::from_str::<CustomProps>(&extras.value).ok())
                        .unwrap_or_default(),
                )
                .clone()
        };

        for entity in entities.iter() {
            if !reduce_to_root(&children, entity, false, |f, r| f || (root == r)) {
                continue;
            }

            let props = prop(entity).clone();
            let props = reduce_to_root(&children, entity, props, |props, r| {
                let p = prop(r);
                CustomProps {
                    ignore_physics: p.ignore_physics || props.ignore_physics,
                    invisible: p.invisible || props.invisible,
                    sensor: p.sensor || props.sensor,
                    diffuse_transmission: p.diffuse_transmission || props.diffuse_transmission,
                }
            });

            if let Ok(mut light) = lights.get_mut(entity) {
                light.shadows_enabled = true;
                light.range = 1000.0;
                light.radius = 0.25;
            }

            if let Ok(mat) = mats.get(entity) {
                let mat = materials.get_mut(mat).unwrap();
                if mat.emissive.l() != 0.0 {
                    mat.fog_enabled = false;
                }
            }

            if props.invisible || props.sensor {
                commands.entity(entity).insert(Visibility::Hidden);
            }

            if props.diffuse_transmission {
                commands.entity(entity).insert(TransmittedShadowReceiver);
            }

            if !props.ignore_physics {
                if let Ok((aabb, transform)) = aabbs.get(entity) {
                    let lp1 = aabb.center - aabb.half_extents;
                    let lp2 = aabb.center + aabb.half_extents;

                    let gp1 = transform.transform_point(lp1.into());
                    let gp2 = transform.transform_point(lp2.into());

                    if gp1.min(gp2).z <= 0.0 && gp1.max(gp2).z >= 0.0 {
                        let mut new_entity = commands.spawn((
                            TransformBundle::from(Transform::from_translation(Vec3::from((
                                aabb.center.xy(),
                                0.0,
                            )))),
                            Collider::cuboid(aabb.half_extents.x, aabb.half_extents.y),
                        ));
                        new_entity.set_parent(entity);
                        if props.sensor {
                            new_entity.insert((Sensor, ActiveEvents::COLLISION_EVENTS));
                        }
                        if let Ok(name) = names.get(entity) {
                            new_entity.insert(name.clone());
                        }
                    }
                }
            }
        }

        scene.on_ready.take().unwrap()(
            &mut commands,
            GameSceneData {
                animations: gltf.named_animations.clone().into_iter().collect(),
            },
        );
        commands.entity(root).remove::<LoadGameScene>();
    }
}
