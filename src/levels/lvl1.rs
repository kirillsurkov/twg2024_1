use anyhow::Result;
use bevy::prelude::*;

use crate::{
    components::{
        code::Code, gate::Gate, loading::Loading, security_camera::SecurityCamera, socket::Socket,
        switch::Switch,
    },
    game_scene::{GameScene, GameSceneData},
    handle_errors,
    player::Player,
    utils::reduce_to_root,
    GameState,
};

use super::{GameLevel, LoadLevel};

struct Entities {
    cam1: Entity,
    switch1: Entity,
    switch2: Entity,
    gate1: Entity,
    code1: Entity,
    socket1: Entity,
    socket2: Entity,
}

#[derive(Resource)]
pub struct Level1 {
    scene_data: GameSceneData,
    entities: Option<Entities>,
}

impl GameScene for Level1 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            entities: None,
        }
    }
}

impl GameLevel for Level1 {
    fn build(state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state.clone()), setup);
        app.add_systems(OnExit(state.clone()), cleanup);
        app.add_systems(
            Update,
            ((
                ready.run_if(resource_added::<Level1>()),
                (process_sensors, process_animations.pipe(handle_errors))
                    .before(ready)
                    .run_if(resource_exists::<Player>())
                    .run_if(resource_exists::<Level1>())
                    .run_if(not(any_with_component::<Loading>())),
            )
                .run_if(in_state(state.clone())),),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level1>("lvl1.glb", 1));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level1>();
}

fn ready(
    mut commands: Commands,
    mut level: ResMut<Level1>,
    entities: Query<(Entity, &Name)>,
    children: Query<&Parent>,
) {
    let mut cam1 = None;
    let mut switch1 = None;
    let mut switch2 = None;
    let mut gate1 = None;
    let mut code1 = None;
    let mut socket1 = None;
    let mut socket2 = None;

    let root = level.scene_data.root;
    for (entity, name) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        let mut entity = commands.entity(entity);
        match name.as_str() {
            "camera.1" => cam1 = Some(entity.insert((Loading, SecurityCamera::new())).id()),
            "switch.1" => {
                switch1 = Some(
                    entity
                        .insert((Loading, Switch::new(&level.scene_data.animations)))
                        .id(),
                );
            }
            "switch.2" => {
                switch2 = Some(
                    entity
                        .insert((Loading, Switch::new(&level.scene_data.animations)))
                        .id(),
                );
            }
            "gate.1" => {
                gate1 = Some(
                    entity
                        .insert((Loading, Gate::new(&level.scene_data.animations)))
                        .id(),
                )
            }
            "code.1" => code1 = Some(entity.insert((Loading, Code::new(1234))).id()),
            "socket_start.1" => socket1 = Some(entity.insert((Loading, Socket::new(true))).id()),
            "socket_end.1" => socket2 = Some(entity.insert((Loading, Socket::new(false))).id()),
            _ => {}
        };
    }

    level.entities = Some(Entities {
        cam1: cam1.unwrap(),
        switch1: switch1.unwrap(),
        switch2: switch2.unwrap(),
        gate1: gate1.unwrap(),
        code1: code1.unwrap(),
        socket1: socket1.unwrap(),
        socket2: socket2.unwrap(),
    });
}

fn process_sensors(
    level: Res<Level1>,
    mut sec_cams: Query<&mut SecurityCamera>,
    mut gates: Query<&mut Gate>,
    switches: Query<&Switch>,
    codes: Query<&Code>,
    sockets: Query<&Socket>,
) {
    let Some(entities) = &level.entities else {
        return;
    };

    let mut cam1 = sec_cams.get_mut(entities.cam1).unwrap();
    let mut gate1 = gates.get_mut(entities.gate1).unwrap();
    let switch1 = switches.get(entities.switch1).unwrap();
    let switch2 = switches.get(entities.switch2).unwrap();
    let code1 = codes.get(entities.code1).unwrap();
    let socket1 = sockets.get(entities.socket1).unwrap();
    let socket2 = sockets.get(entities.socket2).unwrap();

    cam1.active = !(switch1.activated() || code1.activated());
    if switch2.activated() && !gate1.opened() {
        gate1.open();
    }
}

fn process_animations(level: Res<Level1>) -> Result<()> {
    Ok(())
}
