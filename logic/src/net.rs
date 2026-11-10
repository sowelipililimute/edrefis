use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use hecs::{DynamicBundle, Entity, EntityBuilder};
use nanoserde::{DeJson, SerJson};

use crate::{
    field::{GameState, Level},
    input::InputState,
    piece::Piece,
    randomizer::Randomizer,
    well::Well,
};

pub trait World {
    fn new() -> Self;
    fn spawn(&mut self, components: impl DynamicBundle) -> Entity;
    fn query<Q: hecs::Query>(&self) -> hecs::QueryBorrow<'_, Q>;
    fn query_mut<Q: hecs::Query>(&mut self) -> hecs::QueryMut<'_, Q>;
    fn insert(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle,
    ) -> Result<(), hecs::NoSuchEntity>;
    fn despawn(&mut self, entity: Entity) -> Result<(), hecs::NoSuchEntity>;
    fn remove_one<T: hecs::Component>(&mut self, entity: Entity) -> Result<T, hecs::ComponentError>;
    fn get<'a, T: hecs::ComponentRef<'a>>(
        &'a self,
        entity: Entity,
    ) -> Result<T::Ref, hecs::ComponentError>;
    fn iter(&self) -> hecs::Iter<'_>;
    fn tick(&self) -> Tick;
    fn increment_tick(&mut self);
    fn dirty<Q: hecs::Query>(&self, entity: Entity);
}

pub struct ClientWorld {
    world: hecs::World,
    tick: Tick,
    registry: NetComponentRegistry,
}

impl World for ClientWorld {
    fn new() -> Self {
        ClientWorld {
            world: hecs::World::new(),
            tick: Tick(0),
            registry: NetComponentRegistry::new_with_all_components(),
        }
    }
    fn spawn(&mut self, components: impl DynamicBundle) -> Entity {
        self.world.spawn(components)
    }
    fn query<Q: hecs::Query>(&self) -> hecs::QueryBorrow<'_, Q> {
        self.world.query()
    }
    fn query_mut<Q: hecs::Query>(&mut self) -> hecs::QueryMut<'_, Q> {
        self.world.query_mut()
    }
    fn insert(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle,
    ) -> Result<(), hecs::NoSuchEntity> {
        self.world.insert(entity, components)
    }
    fn despawn(&mut self, entity: Entity) -> Result<(), hecs::NoSuchEntity> {
        self.world.despawn(entity)
    }
    fn remove_one<T: hecs::Component>(&mut self, entity: Entity) -> Result<T, hecs::ComponentError> {
        self.world.remove_one::<T>(entity)
    }
    fn get<'a, T: hecs::ComponentRef<'a>>(
        &'a self,
        entity: Entity,
    ) -> Result<T::Ref, hecs::ComponentError> {
        self.world.get::<T>(entity)
    }
    fn iter(&self) -> hecs::Iter<'_> {
        self.world.iter()
    }
    fn tick(&self) -> Tick {
        self.tick
    }
    fn increment_tick(&mut self) {
        self.tick = Tick(self.tick.0 + 1);
    }
    fn dirty<Q: hecs::Query>(&self, _entity: Entity) {}
}

impl ClientWorld {
    pub fn apply_states(
        &mut self,
        server_to_client_ids: &mut HashMap<NetId, Entity>,
        states: &States,
    ) {
        for (suid, component) in &states.removed {
            let Some(cuid) = server_to_client_ids.get(suid).copied() else {
                continue;
            };

            match component {
                None => {
                    let _ = self.world.despawn(cuid);
                    server_to_client_ids.remove(suid);
                }
                Some(component) => {
                    if let Some(registration) = self
                        .registry
                        .components
                        .values()
                        .find(|registration| registration.component_id == *component)
                    {
                        (registration.remove_component)(&mut self.world, cuid);
                    }
                }
            }
        }

        let mut builders = HashMap::<NetId, EntityBuilder>::new();

        for registration in self.registry.components.values() {
            ((registration.deserialize_components)(
                registration.component_id,
                &mut builders,
                states,
            ));
        }

        for (suid, builder) in &mut builders {
            if let Some(cuid) = server_to_client_ids.get(suid) {
                self.world.insert(*cuid, builder.build()).unwrap();
            } else {
                builder.add(Remote(*suid));
                let cuid = self.world.spawn(builder.build());
                server_to_client_ids.insert(*suid, cuid);
            }
        }
    }
}

pub struct ServerWorld {
    world: hecs::World,
    next_uid: NetId,
    tick: Tick,
    registry: NetComponentRegistry,
    tombstones: Vec<Tombstone>,
}

impl World for ServerWorld {
    fn new() -> Self {
        ServerWorld {
            world: hecs::World::new(),
            next_uid: NetId(0),
            tick: Tick(0),
            registry: NetComponentRegistry::new_with_all_components(),
            tombstones: Vec::new(),
        }
    }
    fn spawn(&mut self, components: impl DynamicBundle) -> Entity {
        let mut builder = EntityBuilder::new();

        for (type_id, registration) in &self.registry.components {
            if components.with_ids(|types| types.contains(type_id)) {
                (registration.dirty_spawn_component)(&mut builder, self.tick);
            }
        }

        builder.add_bundle(components).add(Replicated {
            id: self.next_uid,
            spawned_at: self.tick,
        });

        self.next_uid = NetId(self.next_uid.0 + 1);
        self.world.spawn(builder.build())
    }
    fn query<Q: hecs::Query>(&self) -> hecs::QueryBorrow<'_, Q> {
        self.world.query()
    }
    fn query_mut<Q: hecs::Query>(&mut self) -> hecs::QueryMut<'_, Q> {
        self.world.query_mut()
    }
    fn insert(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle,
    ) -> Result<(), hecs::NoSuchEntity> {
        let mut builder = EntityBuilder::new();

        for (type_id, registration) in &self.registry.components {
            if components.with_ids(|types| types.contains(type_id)) {
                (registration.dirty_spawn_component)(&mut builder, self.tick);
            }
        }

        builder.add_bundle(components);

        self.world.insert(entity, builder.build())
    }
    fn despawn(&mut self, entity: Entity) -> Result<(), hecs::NoSuchEntity> {
        let id = self.net_id(entity);
        self.world.despawn(entity)?;

        if let Some(id) = id {
            self.tombstones.push(Tombstone {
                id,
                component: None,
                at: self.tick,
            });
        }

        Ok(())
    }
    fn remove_one<T: hecs::Component>(&mut self, entity: Entity) -> Result<T, hecs::ComponentError> {
        let component = self.world.remove_one::<T>(entity)?;
        let registration = self.registry.components.get(&TypeId::of::<T>());

        if let (Some(id), Some(registration)) = (self.net_id(entity), registration) {
            let _ = self.world.remove_one::<ReplicatedComp<T>>(entity);
            self.tombstones.push(Tombstone {
                id,
                component: Some(registration.component_id),
                at: self.tick,
            });
        }

        Ok(component)
    }
    fn get<'a, T: hecs::ComponentRef<'a>>(
        &'a self,
        entity: Entity,
    ) -> Result<T::Ref, hecs::ComponentError> {
        self.world.get::<T>(entity)
    }
    fn iter(&self) -> hecs::Iter<'_> {
        self.world.iter()
    }
    fn tick(&self) -> Tick {
        self.tick
    }
    fn increment_tick(&mut self) {
        self.tick = Tick(self.tick.0 + 1);
    }
    fn dirty<Q: hecs::Query>(&self, entity: Entity) {
        <Q::Fetch as hecs::Fetch>::for_each_borrow(|type_id, _unique| {
            if let Some(registration) = self.registry.components.get(&type_id) {
                (registration.dirty_existing_component)(&self.world, entity, self.tick);
            }
        });
    }
}

impl ServerWorld {
    pub fn net_id(&self, entity: Entity) -> Option<NetId> {
        self.world
            .get::<&Replicated>(entity)
            .ok()
            .map(|replicated| replicated.id)
    }

    pub fn trim_tombstones(&mut self, oldest_acked: Tick) {
        self.tombstones.retain(|tombstone| tombstone.at > oldest_acked);
    }

    pub fn serialize(&self, since: Option<Tick>) -> States {
        let removed = match since {
            Some(since) => self
                .tombstones
                .iter()
                .filter(|tombstone| tombstone.at > since)
                .map(|tombstone| (tombstone.id, tombstone.component))
                .collect(),
            None => Vec::new(),
        };
        let mut states = States {
            entity_states: HashMap::new(),
            removed,
        };

        for registration in self.registry.components.values() {
            (registration.serialize_components)(
                registration.component_id,
                since,
                &self.world,
                &mut states,
            );
        }

        states
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, PartialOrd, Ord, SerJson, DeJson, Default)]
pub struct Tick(pub u32);

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, SerJson, DeJson)]
pub struct NetId(pub u32);

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, SerJson, DeJson)]
pub struct CompId(pub u32);

#[derive(Debug)]
pub struct Replicated {
    id: NetId,
    spawned_at: Tick,
}

#[derive(Debug)]
pub struct Remote(pub NetId);

#[derive(Debug)]
pub struct ReplicatedComp<TComp> {
    pub changed_at: Tick,
    _phantom: PhantomData<TComp>,
}

#[derive(Debug, Clone, Copy)]
struct Tombstone {
    id: NetId,
    component: Option<CompId>,
    at: Tick,
}

#[derive(SerJson, DeJson, Clone)]
pub struct States {
    pub entity_states: HashMap<NetId, HashMap<CompId, String>>,
    pub removed: Vec<(NetId, Option<CompId>)>,
}

fn hash(string: &str) -> CompId {
    let mut hash: u32 = 0;

    for byte in string.as_bytes() {
        hash = (*byte as u32)
            .wrapping_add(hash << 6)
            .wrapping_add(hash << 16)
            .wrapping_sub(hash);
    }

    CompId(hash)
}

type DirtyExistingCallback = fn(&hecs::World, Entity, Tick) -> ();

fn make_dirty_existing_callback<TComp: Send + Sync + 'static>() -> DirtyExistingCallback {
    fn dirty_callback<TComp: Send + Sync + 'static>(
        world: &hecs::World,
        entity: Entity,
        tick: Tick,
    ) {
        if let Ok(mut rc) = world.get::<&mut ReplicatedComp<TComp>>(entity) {
            rc.changed_at = tick;
        }
    }
    dirty_callback::<TComp>
}

type DirtySpawnCallback = fn(&mut EntityBuilder, Tick) -> ();

fn make_dirty_spawn_callback<TComp: Send + Sync + 'static>() -> DirtySpawnCallback {
    fn dirty_callback<TComp: Send + Sync + 'static>(builder: &mut EntityBuilder, tick: Tick) {
        builder.add(ReplicatedComp::<TComp> {
            changed_at: tick,
            _phantom: PhantomData,
        });
    }
    dirty_callback::<TComp>
}

type RemoveCallback = fn(&mut hecs::World, Entity) -> ();

fn make_remove_callback<TComp: Send + Sync + 'static>() -> RemoveCallback {
    fn remove_callback<TComp: Send + Sync + 'static>(world: &mut hecs::World, entity: Entity) {
        let _ = world.remove_one::<TComp>(entity);
    }
    remove_callback::<TComp>
}

type SerializeCallback = fn(CompId, Option<Tick>, &hecs::World, &mut States) -> ();

fn make_serialize_callback<TComp: SerJson + Send + Sync + 'static>() -> SerializeCallback {
    fn serialize_callback<TComp: SerJson + Send + Sync + 'static>(
        component_id: CompId,
        since: Option<Tick>,
        world: &hecs::World,
        states: &mut States,
    ) {
        for (replicated, component, tracker) in world
            .query::<(&Replicated, &TComp, &ReplicatedComp<TComp>)>()
            .iter()
        {
            if let Some(since) = since {
                if replicated.spawned_at <= since && tracker.changed_at <= since {
                    continue;
                }
            }


            let entity_components = states
                .entity_states
                .entry(replicated.id)
                .or_insert(HashMap::new());

            entity_components.insert(component_id, component.serialize_json());
        }
    }
    serialize_callback::<TComp>
}

type DeserializeCallback = fn(CompId, &mut HashMap<NetId, EntityBuilder>, &States) -> ();

fn make_deserialize_callback<TComp: DeJson + Send + Sync + 'static>() -> DeserializeCallback {
    fn deserialize_callback<TComp: DeJson + Send + Sync + 'static>(
        component_id: CompId,
        builders: &mut HashMap<NetId, EntityBuilder>,
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
    component_id: CompId,
    dirty_existing_component: DirtyExistingCallback,
    dirty_spawn_component: DirtySpawnCallback,
    remove_component: RemoveCallback,
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
                dirty_existing_component: make_dirty_existing_callback::<TComp>(),
                dirty_spawn_component: make_dirty_spawn_callback::<TComp>(),
                remove_component: make_remove_callback::<TComp>(),
                serialize_components: make_serialize_callback::<TComp>(),
                deserialize_components: make_deserialize_callback::<TComp>(),
            },
        );
    }

    fn new_with_all_components() -> NetComponentRegistry {
        let mut registry = NetComponentRegistry::new();
        registry.register::<Well>("Well");
        registry.register::<Randomizer>("Randomizer");
        registry.register::<Level>("Level");
        registry.register::<GameState>("GameState");
        registry.register::<Piece>("ActivePiece");
        registry.register::<InputState>("InputState");
        registry
    }
}
