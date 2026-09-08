use hecs::{Entity, World};
use logic::field::spawn_field;
use logic::net::*;
use std::collections::HashMap;

#[test]
fn state_sync_basics() {
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
