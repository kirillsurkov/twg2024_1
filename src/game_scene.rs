use std::collections::HashMap;

use bevy::{
    gltf::{Gltf, GltfExtras},
    pbr::{ExtendedMaterial, NotShadowCaster, NotShadowReceiver, OpaqueRendererMethod, TransmittedShadowReceiver},
    prelude::*,
    render::{mesh::VertexAttributeValues, primitives::Aabb, view::RenderLayers},
};
use bevy_rapier2d::geometry::{ActiveEvents, Collider, Sensor};
use serde::Deserialize;

use crate::{materials::paint_material::PaintMaterial, utils::reduce_to_root};

pub struct GameSceneData {
    pub root: Entity,
    pub animations: HashMap<String, Handle<AnimationClip>>,
}

pub trait GameScene {
    fn from_scene_data(data: GameSceneData) -> Self;
}

pub struct GameScenePlugin;

impl Plugin for GameScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load.run_if(any_with_component::<LoadGameScene>()));
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
    #[serde(default)]
    no_shadow: bool,
    #[serde(default)]
    color: Vec3,
    #[serde(default)]
    complex_physics: bool,
    #[serde(default)]
    text: bool,
}

fn load(
    mut commands: Commands,
    mut scenes: Query<(Entity, &mut LoadGameScene)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut PointLight>,
    mut text_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, PaintMaterial>>>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    meshes: Res<Assets<Mesh>>,
    entities: Query<Entity>,
    children: Query<&Parent>,
    extras: Query<&GltfExtras>,
    material_hs: Query<&Handle<StandardMaterial>>,
    mesh_hs: Query<&Handle<Mesh>>,
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

            let props = reduce_to_root(&children, entity, prop(entity).clone(), |props, r| {
                let p = prop(r);
                CustomProps {
                    ignore_physics: p.ignore_physics || props.ignore_physics,
                    invisible: p.invisible || props.invisible,
                    sensor: p.sensor || props.sensor,
                    diffuse_transmission: p.diffuse_transmission || props.diffuse_transmission,
                    no_shadow: p.no_shadow || props.no_shadow,
                    color: props.color,
                    complex_physics: p.complex_physics || props.complex_physics,
                    text: p.text || props.text,
                }
            });

            if let Ok(mut light) = lights.get_mut(entity) {
                light.shadows_enabled = true;
                light.range = 1000.0;
                light.radius = 0.25;
            }

            if let Ok(material) = material_hs.get(entity) {
                let material = materials.get_mut(material).unwrap();

                if props.text {
                    let mut base = material.clone();
                    base.alpha_mode = AlphaMode::Blend;
                    base.opaque_render_method = OpaqueRendererMethod::Forward;
                    let h = text_materials.add(ExtendedMaterial {
                        base,
                        extension: PaintMaterial {},
                    });
                    commands.entity(entity).remove::<Handle<StandardMaterial>>();
                    commands.entity(entity).insert((h, RenderLayers::layer(1)));
                }
            }

            if props.invisible || props.sensor {
                commands.entity(entity).insert(Visibility::Hidden);
            }

            if props.diffuse_transmission {
                commands.entity(entity).insert(TransmittedShadowReceiver);
            }

            if props.no_shadow {
                commands
                    .entity(entity)
                    .insert((NotShadowCaster, NotShadowReceiver));
            }

            if !props.ignore_physics {
                let new_entity = if props.complex_physics {
                    if let Ok(mesh) = mesh_hs.get(entity) {
                        let mesh = meshes.get(mesh).unwrap();
                        let vertices = mesh
                            .attribute(Mesh::ATTRIBUTE_POSITION)
                            .and_then(VertexAttributeValues::as_float3)
                            .unwrap()
                            .into_iter()
                            .map(|[x, y, _]| Vec2::new(*x, *y))
                            .collect();
                        let indices = mesh
                            .indices()
                            .unwrap()
                            .iter()
                            .fold(vec![], |mut acc, v| {
                                match acc.last_mut().and_then(|last: &mut [u32; 4]| {
                                    if last[0] < 3 {
                                        Some(last)
                                    } else {
                                        None
                                    }
                                }) {
                                    Some(last) => {
                                        last[0] += 1;
                                        last[last[0] as usize] = v as u32;
                                    }
                                    None => {
                                        acc.push([1, v as u32, 0, 0]);
                                    }
                                }
                                acc
                            })
                            .into_iter()
                            .map(|[_, x, y, z]| [x, y, z])
                            .collect();

                        Some(commands.spawn((
                            TransformBundle::default(),
                            Collider::trimesh(vertices, indices),
                        )))
                    } else {
                        None
                    }
                } else if let Ok((aabb, transform)) = aabbs.get(entity) {
                    let p1 = transform.transform_point((aabb.center - aabb.half_extents).into());
                    let p2 = transform.transform_point((aabb.center + aabb.half_extents).into());

                    if p1.min(p2).z <= 0.0 && p1.max(p2).z >= 0.0 {
                        Some(commands.spawn((
                            TransformBundle::from(Transform::from_translation(Vec3::from((
                                aabb.center.xy(),
                                0.0,
                            )))),
                            Collider::cuboid(aabb.half_extents.x, aabb.half_extents.y),
                        )))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(mut new_entity) = new_entity {
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

        scene.on_ready.take().unwrap()(
            &mut commands,
            GameSceneData {
                root,
                animations: gltf.named_animations.clone().into_iter().collect(),
            },
        );
        commands.entity(root).remove::<LoadGameScene>();
    }
}
