use std::{collections::HashSet, hash::Hash};

use logic::input::{FrameInputs, Input};

pub trait ClientInputs {
    fn sample(&self) -> FrameInputs;
}

pub struct KeyboardInputs<K: Eq + Hash + Clone> {
    binding: fn(Input) -> K,
    down: HashSet<K>,
}

impl<K: Eq + Hash + Clone> KeyboardInputs<K> {
    pub fn new(binding: fn(Input) -> K) -> Self {
        KeyboardInputs {
            binding,
            down: HashSet::new(),
        }
    }

    pub fn push_key(&mut self, key: K) {
        self.down.insert(key);
    }

    pub fn release_key(&mut self, key: &K) {
        self.down.remove(key);
    }
}

impl<K: Eq + Hash + Clone> ClientInputs for KeyboardInputs<K> {
    fn sample(&self) -> FrameInputs {
        let mut frame = FrameInputs::default();
        for input in Input::ALL {
            let k = (self.binding)(input);
            if self.down.contains(&k) {
                frame = frame.with(input);
            }
        }
        frame
    }
}
