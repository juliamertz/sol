use std::collections::HashMap;
use std::hash::Hash;

pub trait Id {
    fn new(inner: u32) -> Self;
    fn into_inner(self) -> u32;
}

#[macro_export]
macro_rules! id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, ::std::cmp::Eq, ::std::cmp::PartialEq, ::std::hash::Hash)]
        pub struct $name(pub u32);

        impl $name {
            #[allow(unused)]
            pub const DUMMY: Self = Self(u32::MAX);
        }

        impl $crate::interner::Id for $name {
            fn new(inner: u32) -> Self {
                Self(inner)
            }

            fn into_inner(self) -> u32 {
                self.0
            }
        }
    };
}

pub trait Strategy<K, V> {
    fn key_for(&mut self, value: &V) -> K;

    fn default_values() -> Option<HashMap<K, V>> {
        None
    }
}

#[derive(Debug, Default)]
pub struct DefaultStrategy {
    idx: u32,
}

impl<K, V> Strategy<K, V> for DefaultStrategy
where
    K: Id,
{
    fn key_for(&mut self, _value: &V) -> K {
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
        let strategy = S::default();
        let map = S::default_values().unwrap_or_default();
        Self { strategy, map }
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

    pub fn intern(&mut self, value: impl Into<V>) -> K {
        let value = value.into();
        let id = self.strategy.key_for(&value);
        self.map.insert(id, value);
        id
    }
}
