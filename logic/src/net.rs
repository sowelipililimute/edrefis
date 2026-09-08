use std::{any::TypeId, collections::HashMap};

use hecs::{Entity, EntityBuilder, World};
use nanoserde::{DeJson, SerJson};

use crate::{field::GameState, piece::Piece, randomizer::Randomizer, well::Well};

#[derive(Debug)]
pub struct Replicated;

#[derive(Debug)]
pub struct Remote;

#[derive(SerJson, DeJson, Clone)]
pub struct States {
    pub entity_states: HashMap<u32, HashMap<u32, String>>,
}

fn hash(string: &str) -> u32 {
    let mut hash: u32 = 0;

    for byte in string.as_bytes() {
        hash = (*byte as u32)
            .wrapping_add(hash << 6)
            .wrapping_add(hash << 16)
            .wrapping_sub(hash);
    }

    hash
}

type SerializeCallback = fn(u32, &mut World, &mut States) -> ();

fn make_serialize_callback<TComp: SerJson + Send + Sync + 'static>() -> SerializeCallback {
    fn serialize_callback<TComp: SerJson + Send + Sync + 'static>(
        component_id: u32,
        world: &mut World,
        states: &mut States,
    ) {
        for (uid, _, component) in world.query_mut::<(Entity, &Replicated, &TComp)>() {
            let entity_components = states
                .entity_states
                .entry(uid.id())
                .or_insert(HashMap::new());

            entity_components.insert(component_id, component.serialize_json());
        }
    }
    serialize_callback::<TComp>
}

type DeserializeCallback = fn(u32, &mut HashMap<u32, EntityBuilder>, &States) -> ();

fn make_deserialize_callback<TComp: DeJson + Send + Sync + 'static>() -> DeserializeCallback {
    fn deserialize_callback<TComp: DeJson + Send + Sync + 'static>(
        component_id: u32,
        builders: &mut HashMap<u32, EntityBuilder>,
        states: &States,
    ) {
        for (entity, components) in &states.entity_states {
            if let Some(component_str) = components.get(&component_id) {
                let comp: TComp = DeJson::deserialize_json(component_str).unwrap();
                let builder = builders.entry(*entity).or_insert(EntityBuilder::new());
                builder.add(comp);
            }
        }
    }

    deserialize_callback::<TComp>
}

#[derive(Debug)]
struct NetComponentRegistration {
    component_id: u32,
    serialize_components: SerializeCallback,
    deserialize_components: DeserializeCallback,
}

#[derive(Debug)]
pub struct NetComponentRegistry {
    components: HashMap<TypeId, NetComponentRegistration>,
}

impl NetComponentRegistry {
    fn new() -> NetComponentRegistry {
        NetComponentRegistry {
            components: HashMap::new(),
        }
    }

    fn register<TComp: SerJson + DeJson + Send + Sync + 'static>(&mut self, name: &str) {
        self.components.insert(
            TypeId::of::<TComp>(),
            NetComponentRegistration {
                component_id: hash(name),
                serialize_components: make_serialize_callback::<TComp>(),
                deserialize_components: make_deserialize_callback::<TComp>(),
            },
        );
    }

    pub fn new_with_all_components() -> NetComponentRegistry {
        let mut registry = NetComponentRegistry::new();
        registry.register::<Well>("Well");
        registry.register::<Randomizer>("Randomizer");
        registry.register::<u32>("Level");
        registry.register::<GameState>("GameState");
        registry.register::<Piece>("ActivePiece");
        registry
    }

    pub fn serialize_world(&self, world: &mut World) -> States {
        let mut states = States {
            entity_states: HashMap::new(),
        };

        for registration in self.components.values() {
            (registration.serialize_components)(registration.component_id, world, &mut states);
        }

        states
    }

    pub fn apply_states(
        &self,
        world: &mut World,
        server_to_client_ids: &mut HashMap<u32, Entity>,
        states: &States,
    ) {
        let mut builders = HashMap::<u32, EntityBuilder>::new();

        for registration in self.components.values() {
            ((registration.deserialize_components)(
                registration.component_id,
                &mut builders,
                states,
            ));
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
}
