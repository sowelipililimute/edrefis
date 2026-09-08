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
    let registry = NetComponentRegistry::new_with_all_components();

    let states = registry.serialize_world(&mut server_world);

    let mut client_world = World::new();
    let mut server_to_client_ids = HashMap::<u32, Entity>::new();
    registry.apply_states(&mut client_world, &mut server_to_client_ids, &states);

    assert_eq!(server_to_client_ids.len(), 1);
}
