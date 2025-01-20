// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests for the [`TreeArena`].

use tree_arena::*;

#[test]
fn arena_insertions() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.roots_mut();

    // <empty>

    roots.insert(1_u64, 'a');
    roots.insert(2_u64, 'b');
    assert!(roots.item(1_u64).is_some());

    // >-- 1(a)
    //
    // >-- 2(b)

    let mut child_1 = roots.item_mut(1_u64).unwrap();
    child_1.children.insert(3_u64, 'c');
    assert!(child_1.children.item(3_u64).is_some());

    // >-- 1(a) -- 3(c)
    //
    // >-- 2(b)

    let mut child_3 = child_1.children.item_mut(3_u64).unwrap();
    child_3.children.insert(4_u64, 'd');

    // >-- 1(a) -- 3(c) -- 4(d)
    //
    // >-- 2(b)

    let child_2 = tree.find(2_u64).expect("No child 2 found");
    let child_4 = child_2.children.find(4_u64);
    assert!(
        child_4.is_none(),
        "Child 4 should not be descended from Child 2"
    );
}

#[test]
fn arena_item_removal() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.roots_mut();

    // <empty>

    roots.insert(1_u64, 'a');
    roots.insert(2_u64, 'b');

    // >-- 1(a)
    //
    // >-- 2(b)

    let mut child_1 = roots.item_mut(1_u64).unwrap();
    let child_1_item = child_1.item;
    let mut child_3 = child_1.children.insert(3_u64, 'c');

    // >-- 1(a) -- 3(c)
    //
    // >-- 2(b)

    child_3.children.insert(4_u64, 'd');

    // >-- 1(a) -- 3(c) -- 4(d)
    //
    // >-- 2(b)

    let child_3_removed = child_1.children.remove(3_u64).expect("No child 3 found");
    assert_eq!(child_3_removed, 'c', "Expect removal of node 3");

    // >-- 1(a)
    //
    // >-- 2(b)

    // Check that the borrow of child_1.item is still valid.
    *child_1_item = 'X';

    assert!(child_1.children.find(3_u64).is_none());
    assert!(child_1.children.remove(3_u64).is_none());

    assert!(tree.find(4_u64).is_none());
}

#[test]
#[should_panic(expected = "Key already present")]
fn arena_duplicate_insertion() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.roots_mut();
    roots.insert(1_u64, 'a');
    roots.insert(1_u64, 'b');
}

#[test]
fn arena_mutate_parent_and_child_at_once() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.roots_mut();

    let mut node_1 = roots.insert(1_u64, 'a');
    let node_2 = node_1.children.insert(2_u64, 'b');

    // >-- 1(a) -- 2(b)

    let node_1_item = node_1.item;
    let node_2_item = node_2.item;
    *node_1_item = 'c';
    *node_2_item = 'd';
    *node_1_item = 'e';
    *node_2_item = 'f';
    assert_eq!(*node_1_item, 'e');
    assert_eq!(*node_2_item, 'f');
}
