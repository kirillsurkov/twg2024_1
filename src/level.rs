use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    gltf::{Gltf, GltfExtras},
    pbr::{ShadowFilteringMethod, TransmittedShadowReceiver},
    prelude::*,
    render::primitives::Aabb,
};
use bevy_rapier2d::prelude::*;
use serde::Deserialize;

use crate::{handle_errors, player::Player, GameState};

pub trait GameLevel {
    fn on_enter(&self, state: GameState, app: &mut App);
    fn on_exit(&self, state: GameState, app: &mut App);
    fn update(&self, state: GameState, app: &mut App);
}

#[derive(Resource)]
pub struct LevelAnimations {
    pub named: HashMap<String, Handle<AnimationClip>>,
}

#[derive(Resource)]
pub struct LevelLoad {
    gltf: Option<Handle<Gltf>>,
    root: Option<Entity>,
    name: String,
    scene: u32,
}

impl LevelLoad {
    pub fn new(name: &str, scene: u32) -> Self {
        Self {
            gltf: None,
            root: None,
            name: name.to_string(),
            scene,
        }
    }
}

#[derive(Resource)]
pub struct LevelInit;

#[derive(Component)]
pub struct LevelTag;

#[derive(Default)]
pub struct LevelPlugin {
    levels: HashMap<GameState, Arc<Box<dyn GameLevel + Send + Sync>>>,
}

impl LevelPlugin {
    pub fn with_level<T: GameLevel + Send + Sync + 'static>(
        mut self,
        state: GameState,
        level: T,
    ) -> Self {
        self.levels.insert(state, Arc::new(Box::new(level)));
        self
    }
}

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, level_load.run_if(resource_exists::<LevelLoad>()));
        app.add_systems(Update, level_init.run_if(resource_exists::<LevelInit>()));
        for (state, level) in &self.levels {
            level.on_enter(state.clone(), app);
            level.on_exit(state.clone(), app);
            level.update(state.clone(), app);

            app.add_systems(OnEnter(state.clone()), on_enter.pipe(handle_errors))
                .add_systems(OnExit(state.clone()), on_exit.pipe(handle_errors));
        }
    }
}

fn on_enter(mut commands: Commands, asset_server: Res<AssetServer>) -> Result<()> {
    Player::spawn(&mut commands, &asset_server);

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..Default::default()
            },
            transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::BlenderFilmic,
            ..default()
        },
        ShadowFilteringMethod::Castano13,
        BloomSettings {
            ..Default::default()
        },
        FogSettings {
            color: Color::hsl(180.0, 0.8, 0.1),
            directional_light_color: Color::rgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                20.0,
                Color::hsl(180.0, 0.8, 0.3),
                Color::hsl(180.0, 0.8, 0.5),
            ),
        },
    ));

    Ok(())
}

fn on_exit(mut commands: Commands, entities: Query<Entity, With<LevelTag>>) -> Result<()> {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<LevelLoad>();
    commands.remove_resource::<LevelInit>();
    Ok(())
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

fn level_load(
    mut commands: Commands,
    mut level: ResMut<LevelLoad>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    entities: Query<Entity>,
    children: Query<&Parent>,
    extras: Query<&GltfExtras>,
) {
    let gltf = match level.gltf {
        Some(ref gltf) => gltf.clone_weak(),
        None => {
            let handle = asset_server.load(&level.name);
            let handle_weak = handle.clone();
            level.gltf = Some(handle);
            handle_weak
        }
    };

    let Some(gltf) = gltfs.get(gltf) else {
        return;
    };

    let Some(scene) = gltf.scenes.get(level.scene as usize) else {
        return;
    };

    let Some(root) = level.root else {
        level.root = Some(
            commands
                .spawn((
                    LevelTag,
                    CustomProps::default(),
                    SceneBundle {
                        scene: scene.clone_weak(),
                        ..Default::default()
                    },
                ))
                .id(),
        );
        return;
    };

    commands.insert_resource(LevelAnimations {
        named: gltf.named_animations.clone().into_iter().collect(),
    });

    for e in entities.iter() {
        if !reduce_to_root(&children, e, false, |f, r| f || (root == r)) {
            continue;
        }
        commands.entity(e).insert((
            LevelTag,
            extras
                .get(e)
                .ok()
                .and_then(|extras| serde_json::from_str::<CustomProps>(&extras.value).ok())
                .unwrap_or_default(),
        ));
    }

    commands.remove_resource::<LevelLoad>();
    commands.insert_resource(LevelInit);
}

fn level_init(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut PointLight, With<LevelTag>>,
    entities: Query<Entity, With<LevelTag>>,
    mats: Query<&Handle<StandardMaterial>>,
    all_props: Query<&CustomProps>,
    children: Query<&Parent>,
    aabbs: Query<(&Aabb, &GlobalTransform)>,
    names: Query<&Name>,
) {
    for mut light in lights.iter_mut() {
        light.intensity *= 0.002;
        light.shadows_enabled = true;
        light.range = 1000.0;
        light.radius = 0.25;
    }

    for e in entities.iter() {
        if all_props.get(e).is_err() {
            println!("{}", entities.iter().count());
            println!("{}", all_props.iter().count());
        }
        let props = all_props.get(e).unwrap().clone();
        let props = reduce_to_root(&children, e, props, |props, r| {
            let p = all_props.get(r).unwrap();
            CustomProps {
                ignore_physics: p.ignore_physics || props.ignore_physics,
                invisible: p.invisible || props.invisible,
                sensor: p.sensor || props.sensor,
                diffuse_transmission: props.diffuse_transmission,
            }
        });

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
                        LevelTag,
                        RigidBody::Fixed,
                        TransformBundle::from(
                            Transform::from_translation(Vec3::from((gc.xy(), 0.0)))
                                .with_rotation(rotation)
                                .with_scale(scale),
                        ),
                        Collider::cuboid(aabb.half_extents.x, aabb.half_extents.y),
                    ));
                    if props.sensor {
                        new_entity.insert((Sensor, ActiveEvents::COLLISION_EVENTS));
                    }
                    if let Ok(name) = names.get(e) {
                        new_entity.insert(name.clone());
                    }
                }
            }
        }
    }

    commands.remove_resource::<LevelInit>();
}
