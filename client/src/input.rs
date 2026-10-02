use std::{collections::HashSet, hash::Hash};

use logic::input::{Input, InputProvider};

pub struct KeyboardInputs<K> {
    binding: fn(Input) -> K,
    just_pressed: HashSet<K>,
    down: HashSet<K>,
}

impl<K: Eq + Hash> KeyboardInputs<K> {
    pub fn new(binding: fn(Input) -> K) -> Self {
        KeyboardInputs {
            binding,
            just_pressed: HashSet::new(),
            down: HashSet::new(),
        }
    }

    pub fn push_key(&mut self, key: K)
    where
        K: Clone,
    {
        self.just_pressed.insert(key.clone());
        self.down.insert(key);
    }

    pub fn release_key(&mut self, key: &K) {
        self.just_pressed.remove(key);
        self.down.remove(key);
    }
}

impl<K: Eq + Hash> InputProvider for KeyboardInputs<K> {
    fn peek(&mut self) {}

    fn consume(&mut self) {
        self.just_pressed.clear();
    }

    fn key_just_pressed(&self, input: Input) -> bool {
        self.just_pressed.contains(&(self.binding)(input))
    }

    fn key_down(&self, input: Input) -> bool {
        self.down.contains(&(self.binding)(input))
    }
}
