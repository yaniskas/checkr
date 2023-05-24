use std::collections::{BTreeSet, HashMap, BTreeMap};

use std::hash::Hash;

use std::collections::HashSet;

pub trait Add {
    type Item;
    fn add(self, item: Self::Item) -> Self;
}

pub trait AddMany {
    type Item;
    fn add_many(self, items: impl IntoIterator<Item = Self::Item>) -> Self;
}

impl <T> Add for HashSet<T> 
where
    T: Eq + Hash {
    type Item = T;

    fn add(mut self, item: Self::Item) -> Self {
        self.insert(item);
        self
    }
}

impl <T> AddMany for HashSet<T> 
where
    T: Eq + Hash {
    type Item = T;

    fn add_many(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self {
        self.extend(items);
        self
    }
}

impl <T> Add for Vec<T> {
    type Item = T;

    fn add(mut self, item: Self::Item) -> Self {
        self.push(item);
        self
    }
}

impl <T> AddMany for Vec<T> {
    type Item = T;

    fn add_many(mut self, item: impl IntoIterator<Item = Self::Item>) -> Self {
        self.extend(item);
        self
    }
}

impl <T: Ord> Add for BTreeSet<T> {
    type Item = T;

    fn add(mut self, item: Self::Item) -> Self {
        self.insert(item);
        self
    }
}

impl <T: Ord> AddMany for BTreeSet<T> {
    type Item = T;

    fn add_many(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self {
        self.extend(items);
        self
    }
}

impl <K: Ord, V> Add for BTreeMap<K, V> {
    type Item = (K, V);

    fn add(mut self, item: Self::Item) -> Self {
        self.insert(item.0, item.1);
        self
    }
}

impl <K: Ord, V> AddMany for BTreeMap<K, V> {
    type Item = (K, V);

    fn add_many(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self {
        for (k, v) in items {
            self.insert(k, v);
        }
        self
    }
}

impl <K: Eq + Hash, V> AddMany for HashMap<K, V> {
    type Item = (K, V);

    fn add_many(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self {
        for (k, v) in items {
            self.insert(k, v);
        }
        self
    }
}

pub trait Singleton {
    type Item;
    fn singleton(item: Self::Item) -> Self;
}

impl <T: Ord> Singleton for BTreeSet<T> {
    type Item = T;
    fn singleton(item: Self::Item) -> Self {
        BTreeSet::new().add(item)
    }
}

pub trait WithRemoved {
    type Item;
    fn with_removed(self, item: &Self::Item) -> Self;
}

impl <T: Hash + Eq> WithRemoved for HashSet<T> {
    type Item = T;
    fn with_removed(mut self, item: &Self::Item) -> Self {
        self.remove(item);
        self
    }
}

impl <T: Hash + Eq, U> WithRemoved for HashMap<T, U> {
    type Item = T;
    fn with_removed(mut self, item: &Self::Item) -> Self {
        self.remove(item);
        self
    }
}

impl <T: Ord + Eq> WithRemoved for BTreeSet<T> {
    type Item = T;
    fn with_removed(mut self, item: &Self::Item) -> Self {
        self.remove(item);
        self
    }
}

impl <T: Ord + Eq, U> WithRemoved for BTreeMap<T, U> {
    type Item = T;
    fn with_removed(mut self, item: &Self::Item) -> Self {
        self.remove(item);
        self
    }
}

pub trait Repeat {
    type Item: Clone;

    fn repeat(item: &Self::Item, times: usize) -> Self;
}

impl <T: Clone> Repeat for Vec<T> {
    type Item = T;

    fn repeat(item: &Self::Item, times: usize) -> Self {
        let mut vec = Vec::new();
        for _ in 0..times {
            vec.push(item.clone());
        }
        vec
    }
}

impl <T> Singleton for Vec<T> {
    type Item = T;

    fn singleton(item: Self::Item) -> Self {
        vec! [item]
    }
}