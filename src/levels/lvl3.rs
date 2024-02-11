use anyhow::Result;
use bevy::{prelude::*, render::view::NoFrustumCulling};

use crate::{
    components::{
        code::Code, fan::Fan, gate::Gate, loading::Loading, security_camera::SecurityCamera,
        socket::Socket, switch::Switch,
    },
    game_scene::{GameScene, GameSceneData},
    handle_errors,
    player::Player,
    utils::reduce_to_root,
    GameState, Restart,
};

use super::{GameLevel, LoadLevel};

struct Entities {
    socket_end: Entity,
    gate1: Entity,
    switch1: Entity,
    switch2: Entity,
    code1: Entity,
    code2: Entity,
    fan1: Entity,
    fan2: Entity,
    fan3: Entity,
}

#[derive(Resource)]
pub struct Level3 {
    scene_data: GameSceneData,
    entities: Option<Entities>,
}

impl GameScene for Level3 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            entities: None,
        }
    }
}

impl GameLevel for Level3 {
    fn build(state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state.clone()), setup);
        app.add_systems(OnExit(state.clone()), cleanup);
        app.add_systems(
            Update,
            ((
                ready.run_if(resource_added::<Level3>()),
                (process_sensors, process_animations.pipe(handle_errors))
                    .before(ready)
                    .run_if(resource_exists::<Player>())
                    .run_if(resource_exists::<Level3>())
                    .run_if(not(any_with_component::<Loading>())),
            )
                .run_if(in_state(state.clone())),),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level3>("lvl1.glb", 3));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level3>();
}

fn ready(
    mut commands: Commands,
    mut level: ResMut<Level3>,
    entities: Query<(Entity, &Name)>,
    children: Query<&Parent>,
) {
    let mut socket_end = None;
    let mut gate1 = None;
    let mut switch1 = None;
    let mut switch2 = None;
    let mut code1 = None;
    let mut code2 = None;
    let mut fan1 = None;
    let mut fan2 = None;
    let mut fan3 = None;

    let anims = &level.scene_data.animations;

    let root = level.scene_data.root;
    for (entity, name) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        let mut entity = commands.entity(entity);
        match name.as_str() {
            "socket_start.003" => {
                entity.insert((Loading, Socket::new(true)));
            }
            "socket_end.003" => {
                socket_end = Some(entity.insert((Loading, Socket::new(false))).id())
            }
            "gate.002" => gate1 = Some(entity.insert((Loading, Gate::new(anims))).id()),
            "switch.005" => switch1 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "switch.006" => switch2 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "code.004" => code1 = Some(entity.insert((Loading, Code::new(3028))).id()),
            "code.005" => code2 = Some(entity.insert((Loading, Code::new(8824))).id()),
            "fan.004" => fan1 = Some(entity.insert((Loading, Fan::new())).id()),
            "fan.006" => fan2 = Some(entity.insert((Loading, Fan::new())).id()),
            "fan.008" => fan3 = Some(entity.insert((Loading, Fan::new())).id()),
            _ => {}
        };
    }

    level.entities = Some(Entities {
        socket_end: socket_end.unwrap(),
        gate1: gate1.unwrap(),
        switch1: switch1.unwrap(),
        switch2: switch2.unwrap(),
        code1: code1.unwrap(),
        code2: code2.unwrap(),
        fan1: fan1.unwrap(),
        fan2: fan2.unwrap(),
        fan3: fan3.unwrap(),
    });
}

fn process_sensors(
    mut commands: Commands,
    mut game_state: ResMut<NextState<GameState>>,
    level: Res<Level3>,
    sockets: Query<&Socket>,
    mut gates: Query<&mut Gate>,
    switches: Query<&Switch>,
    codes: Query<&Code>,
    mut fans: Query<&mut Fan>,
) {
    let Some(entities) = &level.entities else {
        return;
    };

    let socket_end = sockets.get(entities.socket_end).unwrap();
    let mut gate1 = gates.get_mut(entities.gate1).unwrap();
    let switch1 = switches.get(entities.switch1).unwrap();
    let switch2 = switches.get(entities.switch2).unwrap();
    let code1 = codes.get(entities.code1).unwrap();
    let code2 = codes.get(entities.code2).unwrap();
    let [mut fan1, mut fan2, mut fan3] = fans
        .get_many_mut([entities.fan1, entities.fan2, entities.fan3])
        .unwrap();

    if code1.activated() {
        fan1.spinning = false;
    }

    if code2.activated() {
        fan2.spinning = false;
    }

    if switch1.activated() && !gate1.opened() {
        gate1.open();
    }

    if switch2.activated() {
        fan3.spinning = false;
    }

    if socket_end.connected() {
        game_state.set(GameState::Level4);
    }
}

fn process_animations(level: Res<Level3>) -> Result<()> {
    Ok(())
}
