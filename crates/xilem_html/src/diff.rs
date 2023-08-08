use std::{cmp::Ordering, collections::BTreeMap, iter::Peekable};

pub fn diff_tree_maps<'a, K: Ord, V: PartialEq>(
    prev: &'a BTreeMap<K, V>,
    next: &'a BTreeMap<K, V>,
) -> impl Iterator<Item = Diff<&'a K, &'a V>> + 'a {
    DiffMapIterator {
        prev: prev.iter().peekable(),
        next: next.iter().peekable(),
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
