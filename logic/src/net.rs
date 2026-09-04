use std::collections::HashMap;

use hecs::{Entity, EntityBuilder, World};
use nanoserde::{DeJson, SerJson};

use crate::{field::GameState, piece::Piece, randomizer::Randomizer, well::Well};

#[derive(Debug)]
struct Replicated;

#[derive(Debug)]
struct Remote;

#[derive(SerJson, DeJson)]
struct States {
    well_states: HashMap<u32, String>,
    randomizer_states: HashMap<u32, String>,
    level_states: HashMap<u32, String>,
    game_states: HashMap<u32, String>,
    active_piece_states: HashMap<u32, String>,
}

fn gather_states(world: &mut World) -> States {
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

fn apply_states(
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

mod tests {
    use super::*;
    use crate::field::spawn_field;
    use hecs::World;

    #[test]
    fn basics() {
        let mut server_world = World::new();
        let server_field = spawn_field(&mut server_world);
        server_world
            .insert(server_field, (Replicated,))
            .expect("should be able to mark it as replicated");

        let states = gather_states(&mut server_world);
        assert_eq!(states.well_states.len(), 1);
        assert_eq!(states.randomizer_states.len(), 1);
        assert_eq!(states.level_states.len(), 1);
        assert_eq!(states.game_states.len(), 1);
        assert_eq!(states.active_piece_states.len(), 1);

        let mut client_world = World::new();
        let mut server_to_client_ids = HashMap::<u32, Entity>::new();
        apply_states(&states, &mut server_to_client_ids, &mut client_world);

        assert_eq!(server_to_client_ids.len(), 1);
    }
}
