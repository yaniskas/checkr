use std::collections::{BTreeSet, HashMap};

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