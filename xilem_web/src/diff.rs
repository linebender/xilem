// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::{cmp::Ordering, iter::Peekable};

/// Computes the diff between two `Iterators` that have a key value mapping and are ordered by key (e.g. a BTreeMap)
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use std::collections::BTreeMap;
/// use crate::diff::{Diff, diff_kv_iterables};
///
/// let mut old = BTreeMap::default();
/// old.insert("c", 3);
/// old.insert("b", 2);
/// old.insert("a", 1);
///
/// let mut new = BTreeMap::default();
/// new.insert("c", 4);
/// new.insert("d", 2);
/// new.insert("a", 1);
///
/// let mut diff = diff_kv_iterables(&old, &new);
///
/// assert!(matches!(diff.next(), Some(Diff::Remove(&"b"))));
/// assert!(matches!(diff.next(), Some(Diff::Change(&"c", 4))));
/// assert!(matches!(diff.next(), Some(Diff::Add(&"d", 2))));
/// assert!(diff.next().is_none());
/// ```
pub fn diff_kv_iterables<'a, II, K, V>(
    prev: II,
    next: II,
) -> impl Iterator<Item = Diff<&'a K, &'a V>> + 'a
where
    K: Ord + 'a,
    V: PartialEq + 'a,
    II: IntoIterator<Item = (&'a K, &'a V)> + 'a,
{
    DiffMapIterator {
        prev: prev.into_iter().peekable(),
        next: next.into_iter().peekable(),
    }
}

/// An iterator that compares two ordered maps (like a `BTreeMap`) and outputs a `Diff` for each added, removed or changed key/value pair)
struct DiffMapIterator<'a, K: 'a, V: 'a, I: Iterator<Item = (&'a K, &'a V)>> {
    prev: Peekable<I>,
    next: Peekable<I>,
}

impl<'a, K: Ord + 'a, V: PartialEq, I: Iterator<Item = (&'a K, &'a V)>> Iterator
    for DiffMapIterator<'a, K, V, I>
{
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! tree_map {
        (@single $($x:tt)*) => (());
        (@count $($rest:expr),*) => (<[()]>::len(&[$(tree_map!(@single $rest)),*]));

        ($($key:expr => $value:expr,)+) => { tree_map!($($key => $value),+) };
        ($($key:expr => $value:expr),*) => {{
            let mut _map = ::std::collections::BTreeMap::new();
            $(
                let _ = _map.insert($key, $value);
            )*
            _map
        }};
    }

    #[test]
    fn maps_are_equal() {
        let map = tree_map!("an-entry" => 1, "another-entry" => 42);
        let map_same = tree_map!("another-entry" => 42, "an-entry" => 1);
        assert!(diff_kv_iterables(&map, &map_same).next().is_none());
    }

    #[test]
    fn new_map_has_additions() {
        let map = tree_map!("an-entry" => 1);
        let map_new = tree_map!("an-entry" => 1, "another-entry" => 42);
        let mut diff = diff_kv_iterables(&map, &map_new);
        assert!(matches!(
            diff.next(),
            Some(Diff::Add(&"another-entry", &42))
        ));
        assert!(diff.next().is_none());
    }

    #[test]
    fn new_map_has_removal() {
        let map = tree_map!("an-entry" => 1, "another-entry" => 42);
        let map_new = tree_map!("an-entry" => 1);
        let mut diff = diff_kv_iterables(&map, &map_new);
        assert!(matches!(diff.next(), Some(Diff::Remove(&"another-entry"))));
        assert!(diff.next().is_none());
    }

    #[test]
    fn new_map_has_removal_and_addition() {
        let map = tree_map!("an-entry" => 1, "another-entry" => 42);
        let map_new = tree_map!("an-entry" => 1, "other-entry" => 2);
        let mut diff = diff_kv_iterables(&map, &map_new);
        assert!(matches!(diff.next(), Some(Diff::Remove(&"another-entry"))));
        assert!(matches!(diff.next(), Some(Diff::Add(&"other-entry", &2))));
        assert!(diff.next().is_none());
    }

    #[test]
    fn new_map_changed() {
        let map = tree_map!("an-entry" => 1, "another-entry" => 42);
        let map_new = tree_map!("an-entry" => 2, "other-entry" => 2);
        let mut diff = diff_kv_iterables(&map, &map_new);
        assert!(matches!(diff.next(), Some(Diff::Change(&"an-entry", 2))));
        assert!(matches!(diff.next(), Some(Diff::Remove(&"another-entry"))));
        assert!(matches!(diff.next(), Some(Diff::Add(&"other-entry", &2))));
        assert!(diff.next().is_none());
    }
}
