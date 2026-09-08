use std::collections::HashMap;

use hecs::{Entity, EntityBuilder, World};
use nanoserde::{DeJson, SerJson};

use crate::{field::GameState, piece::Piece, randomizer::Randomizer, well::Well};

#[derive(Debug)]
pub struct Replicated;

#[derive(Debug)]
pub struct Remote;

#[derive(SerJson, DeJson)]
pub struct States {
    pub well_states: HashMap<u32, String>,
    pub randomizer_states: HashMap<u32, String>,
    pub level_states: HashMap<u32, String>,
    pub game_states: HashMap<u32, String>,
    pub active_piece_states: HashMap<u32, String>,
}

pub fn gather_states(world: &mut World) -> States {
    let mut well_states = HashMap::<u32, String>::new();
    let mut randomizer_states = HashMap::<u32, String>::new();
    let mut level_states = HashMap::<u32, String>::new();
    let mut game_states = HashMap::<u32, String>::new();
    let mut active_piece_states = HashMap::<u32, String>::new();

    for (uid, _, well) in world.query_mut::<(Entity, &Replicated, &Well)>() {
        well_states.insert(uid.id(), well.serialize_json());
    }

    for (uid, _, randomizer) in world.query_mut::<(Entity, &Replicated, &Randomizer)>() {
        randomizer_states.insert(uid.id(), randomizer.serialize_json());
    }

    for (uid, _, level) in world.query_mut::<(Entity, &Replicated, &u32)>() {
        level_states.insert(uid.id(), level.serialize_json());
    }

    for (uid, _, game) in world.query_mut::<(Entity, &Replicated, &GameState)>() {
        game_states.insert(uid.id(), game.serialize_json());
    }

    for (uid, _, active_piece) in world.query_mut::<(Entity, &Replicated, &Piece)>() {
        active_piece_states.insert(uid.id(), active_piece.serialize_json());
    }

    States {
        well_states,
        randomizer_states,
        level_states,
        game_states,
        active_piece_states,
    }
}

pub fn apply_states(
    states: &States,
    server_to_client_ids: &mut HashMap<u32, Entity>,
    world: &mut World,
) {
    let mut builders = HashMap::<u32, EntityBuilder>::new();

    for (suid, well) in &states.well_states {
        let well: Well = DeJson::deserialize_json(well).unwrap();
        let builder = builders.entry(*suid).or_insert(EntityBuilder::new());
        builder.add(well);
    }
    for (suid, randomizer) in &states.randomizer_states {
        let randomizer: Randomizer = DeJson::deserialize_json(randomizer).unwrap();
        let builder = builders.entry(*suid).or_insert(EntityBuilder::new());
        builder.add(randomizer);
    }
    for (suid, level) in &states.level_states {
        let level: u32 = DeJson::deserialize_json(level).unwrap();
        let builder = builders.entry(*suid).or_insert(EntityBuilder::new());
        builder.add(level);
    }
    for (suid, game) in &states.game_states {
        let game: GameState = DeJson::deserialize_json(game).unwrap();
        let builder = builders.entry(*suid).or_insert(EntityBuilder::new());
        builder.add(game);
    }
    for (suid, active_piece) in &states.active_piece_states {
        let active_piece: Piece = DeJson::deserialize_json(active_piece).unwrap();
        let builder = builders.entry(*suid).or_insert(EntityBuilder::new());
        builder.add(active_piece);
    }

    for (suid, builder) in &mut builders {
        if let Some(cuid) = server_to_client_ids.get(suid) {
            world.insert(*cuid, builder.build()).unwrap();
        } else {
            builder.add(Remote);
            let cuid = world.spawn(builder.build());
            server_to_client_ids.insert(*suid, cuid);
        }
    }
}
