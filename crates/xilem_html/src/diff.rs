//! Code for taking 2 `BTreeMap`s, and returning an iterator of changes within them.

use std::{
    cmp::Ordering,
    collections::{btree_map, btree_set, BTreeMap, BTreeSet},
    iter::Peekable,
};

pub fn diff_maps<'a, K: Ord, V: PartialEq>(
    prev: &'a BTreeMap<K, V>,
    next: &'a BTreeMap<K, V>,
) -> impl Iterator<Item = Diff<&'a K, &'a V>> + 'a {
    DiffMapIterator {
        prev: prev.iter().peekable(),
        next: next.iter().peekable(),
    }
}

struct DiffMapIterator<'a, K, V> {
    prev: Peekable<btree_map::Iter<'a, K, V>>,
    next: Peekable<btree_map::Iter<'a, K, V>>,
}

impl<'a, K: Ord, V: PartialEq> Iterator for DiffMapIterator<'a, K, V> {
    type Item = Diff<&'a K, &'a V>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.prev.peek(), self.next.peek()) {
                (Some(&(prev_k, prev_v)), Some(&(next_k, next_v))) => match prev_k.cmp(next_k) {
                    Ordering::Less => {
                        self.prev.next();
                        return Some(Diff::Remove(prev_k));
                    }
                    Ordering::Greater => {
                        self.next.next();
                        return Some(Diff::Add(next_k, next_v));
                    }
                    Ordering::Equal => {
                        self.prev.next();
                        self.next.next();
                        if prev_v != next_v {
                            return Some(Diff::Change(next_k, next_v));
                        }
                    }
                },
                (Some(&(prev_k, _)), None) => {
                    self.prev.next();
                    return Some(Diff::Remove(prev_k));
                }
                (None, Some(&(next_k, next_v))) => {
                    self.next.next();
                    return Some(Diff::Add(next_k, next_v));
                }
                (None, None) => return None,
            }
        }
    }
}

pub enum Diff<K, V> {
    Add(K, V),
    Remove(K),
    Change(K, V),
}

pub fn diff_sets<'a, K: Ord>(
    prev: &'a BTreeSet<K>,
    next: &'a BTreeSet<K>,
) -> impl Iterator<Item = DiffSet<&'a K>> + 'a {
    DiffSetIterator {
        prev: prev.iter().peekable(),
        next: next.iter().peekable(),
    }
}

struct DiffSetIterator<'a, K> {
    prev: Peekable<btree_set::Iter<'a, K>>,
    next: Peekable<btree_set::Iter<'a, K>>,
}

impl<'a, K: Ord> Iterator for DiffSetIterator<'a, K> {
    type Item = DiffSet<&'a K>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.prev.peek(), self.next.peek()) {
                (Some(&prev_k), Some(&next_k)) => match prev_k.cmp(next_k) {
                    Ordering::Less => {
                        self.prev.next();
                        return Some(DiffSet::Remove(prev_k));
                    }
                    Ordering::Greater => {
                        self.next.next();
                        return Some(DiffSet::Add(next_k));
                    }
                    Ordering::Equal => {
                        self.prev.next();
                        self.next.next();
                        // continue loop
                    }
                },
                (Some(&prev_k), None) => {
                    self.prev.next();
                    return Some(DiffSet::Remove(prev_k));
                }
                (None, Some(&next_k)) => {
                    self.next.next();
                    return Some(DiffSet::Add(next_k));
                }
                (None, None) => return None,
            }
        }
    }
}

pub enum DiffSet<K> {
    Add(K),
    Remove(K),
}
