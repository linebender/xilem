// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;
use std::fmt;
use std::ops::Index;
use std::vec::Drain;

#[derive(Clone)]
/// Basically an ordered `Map` (similar as `BTreeMap`) with a `Vec` as backend for very few elements
/// As it uses linear search instead of a tree traversal,
/// which seems to be faster for small `n` (currently roughly `n < ~20` for the use case of diffing html attributes)
pub struct VecMap<K, V>(Vec<(K, V)>);

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K: Eq, V: Eq> Eq for VecMap<K, V> {}
impl<K: PartialEq, V: PartialEq> PartialEq for VecMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for VecMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V> VecMap<K, V> {
    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut map = VecMap::default();
    /// map.insert(1, "a");
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + PartialEq,
        Q: PartialEq + ?Sized,
    {
        self.0
            .iter()
            .find_map(|(k, v)| if key.eq(k.borrow()) { Some(v) } else { None })
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut map = VecMap::default();
    /// map.insert(1, "a");
    /// assert!(map.contains_key(&1));
    /// assert!(!map.contains_key(&2));
    /// ```
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut map = VecMap::default();
    /// map.insert(1, "a");
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map[&1], "b");
    /// ```
    // See `get` for implementation notes, this is basically a copy-paste with mut's added
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0
            .iter_mut()
            .find_map(|(k, v)| if key.eq((*k).borrow()) { Some(v) } else { None })
    }

    /// Gets an iterator over the keys of the map, in sorted order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut a = VecMap::default();
    /// a.insert(2, "b");
    /// a.insert(1, "a");
    ///
    /// let keys: Vec<_> = a.keys().cloned().collect();
    /// assert_eq!(keys, [1, 2]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().map(|(name, _)| name)
    }

    /// Gets an iterator over the entries of the map, sorted by key.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut map = VecMap::default();
    /// map.insert(3, "c");
    /// map.insert(2, "b");
    /// map.insert(1, "a");
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{key}: {value}");
    /// }
    ///
    /// let (first_key, first_value) = map.iter().next().unwrap();
    /// assert_eq!((*first_key, *first_value), (1, "a"));
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    /// Clears the map, returning all key-value pairs as an iterator. Keeps the
    /// allocated memory for reuse.
    ///
    /// If the returned iterator is dropped before being fully consumed, it
    /// drops the remaining key-value pairs. The returned iterator keeps a
    /// mutable borrow on the map to optimize its implementation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::vecmap::VecMap;
    ///
    /// let mut a = VecMap::default();
    /// a.insert(1, "a");
    /// a.insert(2, "b");
    ///
    /// for (k, v) in a.drain().take(1) {
    ///     assert!(k == 1 || k == 2);
    ///     assert!(v == "a" || v == "b");
    /// }
    ///
    /// assert!(a.is_empty());
    /// ```
    pub fn drain(&mut self) -> Drain<'_, (K, V)> {
        self.0.drain(..)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut map = VecMap::default();
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Some("b"));
    /// assert_eq!(map[&37], "c");
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Ord,
    {
        match self.0.binary_search_by_key(&&key, |(n, _)| n) {
            Ok(pos) => {
                let mut val = (key, value);
                std::mem::swap(&mut self.0[pos], &mut val);
                Some(val.1)
            }
            Err(pos) => {
                self.0.insert(pos, (key, value));
                None
            }
        }
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut map = VecMap::default();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        // TODO not sure whether just a simple find is better here? Probably needs more benching
        match self.0.binary_search_by_key(&key, |(k, _)| k.borrow()) {
            Ok(pos) => Some(self.0.remove(pos).1),
            Err(_) => None,
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut a = VecMap::default();
    /// assert!(a.is_empty());
    /// a.insert(1, "a");
    /// assert!(!a.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// # use crate::vecmap::VecMap;
    /// let mut a = VecMap::default();
    /// assert_eq!(a.len(), 0);
    /// a.insert(1, "a");
    /// assert_eq!(a.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `Vec<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Reserves the minimum capacity for at least `additional` more elements to
    /// be inserted in the given `VecMap<K, V>`. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer [`reserve`] if future insertions are expected.
    ///
    /// [`reserve`]: VecMap::reserve
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    pub fn reserve_exact(&mut self, additional: usize) {
        self.0.reserve_exact(additional);
    }
}

impl<K, Q, V> Index<&Q> for VecMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the `VecMap`.
    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<'a, K, V> IntoIterator for &'a VecMap<K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, V)>, fn(&'a (K, V)) -> (&'a K, &'a V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(k, v)| (k, v))
    }
}

impl<'a, K, V> IntoIterator for &'a mut VecMap<K, V> {
    type Item = (&'a mut K, &'a mut V);

    type IntoIter = std::iter::Map<
        std::slice::IterMut<'a, (K, V)>,
        fn(&'a mut (K, V)) -> (&'a mut K, &'a mut V),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut().map(|(k, v)| (k, v))
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);

    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// Basically all the doc tests from the rustdoc examples above, to avoid having to expose this module (pub)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let mut map = VecMap::default();
        map.insert(1, "a");
        assert_eq!(map.get(&1), Some(&"a"));
        assert_eq!(map.get(&2), None);
    }

    #[test]
    fn contains_key() {
        let mut map = VecMap::default();
        map.insert(1, "a");
        assert!(map.contains_key(&1));
        assert!(!map.contains_key(&2));
    }

    #[test]
    fn get_mut() {
        let mut map = VecMap::default();
        map.insert(1, "a");
        if let Some(x) = map.get_mut(&1) {
            *x = "b";
        }
        assert_eq!(map[&1], "b");
    }

    #[test]
    fn keys() {
        let mut a = VecMap::default();
        a.insert(2, "b");
        a.insert(1, "a");
        let keys: Vec<_> = a.keys().cloned().collect();
        assert_eq!(keys, [1, 2]);
    }

    #[test]
    fn iter() {
        let mut map = VecMap::default();
        map.insert(3, "c");
        map.insert(2, "b");
        map.insert(1, "a");
        for (key, value) in map.iter() {
            println!("{key}: {value}");
        }
        let (first_key, first_value) = map.iter().next().unwrap();
        assert_eq!((*first_key, *first_value), (1, "a"));
    }

    #[test]
    fn drain() {
        let mut a = VecMap::default();
        a.insert(1, "a");
        a.insert(2, "b");

        for (k, v) in a.drain().take(1) {
            assert!(k == 1 || k == 2);
            assert!(v == "a" || v == "b");
        }

        assert!(a.is_empty());
    }

    #[test]
    fn insert() {
        let mut map = VecMap::default();

        assert_eq!(map.insert(37, "a"), None);
        assert!(!map.is_empty());

        map.insert(37, "b");
        assert_eq!(map.insert(37, "c"), Some("b"));
        assert_eq!(map[&37], "c");
    }

    #[test]
    fn remove() {
        let mut map = VecMap::default();
        map.insert(1, "a");
        assert_eq!(map.remove(&1), Some("a"));
        assert_eq!(map.remove(&1), None);
    }

    #[test]
    fn is_empty() {
        let mut a = VecMap::default();
        assert!(a.is_empty());
        a.insert(1, "a");
        assert!(!a.is_empty());
    }

    #[test]
    fn len() {
        let mut a = VecMap::default();
        assert_eq!(a.len(), 0);
        a.insert(1, "a");
        assert_eq!(a.len(), 1);
    }
}
