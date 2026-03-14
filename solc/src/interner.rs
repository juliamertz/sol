use std::collections::HashMap;
use std::hash::Hash;

pub trait Id {
    fn new(inner: u32) -> Self;
}

#[macro_export]
macro_rules! id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, ::std::cmp::Eq, ::std::cmp::PartialEq, ::std::hash::Hash)]
        pub struct $name(pub u32);

        impl $name {
            pub const DUMMY: Self = Self(u32::MAX);
        }

        impl $crate::interner::Id for $name {
            fn new(inner: u32) -> Self {
                Self(inner)
            }
        }
    };
}

pub trait Strategy<K, V> {
    fn id_for(&mut self, value: &V) -> K;
}

#[derive(Debug, Default)]
pub struct DefaultStrategy {
    idx: u32,
}

impl<K, V> Strategy<K, V> for DefaultStrategy
where
    K: Id,
{
    fn id_for(&mut self, _value: &V) -> K {
        let id = K::new(self.idx);
        self.idx += 1;
        id
    }
}

#[derive(Debug)]
pub struct Interner<K, V, S = DefaultStrategy> {
    strategy: S,
    map: HashMap<K, V>,
}

impl<K, V, S> Default for Interner<K, V, S>
where
    S: Strategy<K, V> + Default,
{
    fn default() -> Self {
        Self {
            strategy: Default::default(),
            map: Default::default(),
        }
    }
}

impl<K, V, S> Interner<K, V, S>
where
    K: Id + Hash + Eq + Copy,
    S: Strategy<K, V>,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    pub fn intern(&mut self, value: V) -> K {
        let id = self.strategy.id_for(&value);
        self.map.insert(id, value);
        id
    }
}
