// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::mem;

use tree_arena::*;

#[test]
fn arena_tree_test() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.root_token_mut();
    roots.insert_child(1_u64, 'a');
    roots.insert_child(2_u64, 'b');
    let mut child_1 = roots.get_child_mut(1_u64).expect("No child 1 found");
    child_1.children.insert_child(3_u64, 'c');

    let mut child_3 = child_1
        .children
        .get_child_mut(3_u64)
        .expect("No child 3 found");
    child_3.children.insert_child(4_u64, 'd');

    let child_2 = tree.find(2_u64).expect("No child 2 found");
    let child_4 = child_2.children.find(4_u64);
    assert!(
        child_4.is_none(),
        "Child 4 should not be descended from Child 2"
    );
}

#[test]
fn arena_tree_removal_test() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.root_token_mut();
    roots.insert_child(1_u64, 'a');
    roots.insert_child(2_u64, 'b');
    let mut child_1 = roots.get_child_mut(1_u64).expect("No child 1 found");
    child_1.children.insert_child(3_u64, 'c');

    let mut child_3 = child_1
        .children
        .get_child_mut(3_u64)
        .expect("No child 3 found");
    child_3.children.insert_child(4_u64, 'd');

    let child_3_removed = child_1
        .children
        .remove_child(3_u64)
        .expect("No child 3 found");
    assert_eq!(child_3_removed, 'c', "Expect removal of node 3");

    let no_child_3_removed = child_1.children.remove_child(3_u64);
    assert!(no_child_3_removed.is_none(), "Child 3 was not removed");
}

#[test]
#[should_panic(expected = "Key already present")]
fn arena_tree_duplicate_insertion() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.root_token_mut();
    roots.insert_child(1_u64, 'a');
    roots.insert_child(1_u64, 'b');
}

#[test]
fn parent_child_items() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.root_token_mut();
    roots.insert_child(1_u64, 'a');
    let mut node_1 = roots.get_child_mut(1_u64).expect("No child 1 found");
    node_1.children.insert_child(2_u64, 'b');
    let node_1_item = node_1.item;
    let node_2_item = node_1
        .children
        .get_child_mut(2_u64)
        .expect("No child 2 found")
        .item;
    *node_1_item = 'c';
    *node_2_item = 'd';
    assert_eq!(*node_1_item, 'c', "Node 1 item should be 'c'");
    assert_eq!(*node_2_item, 'd', "Node 2 item should be 'd'");
}

// test creating trees-
// --1(a)--2(b)
//   |
//   3(c)--4(d)
//
// and
//
// --4(e)--3(f)
//   |
//   2(g)--1(h)
//
// and swapping references to the children of 1(a) and 4(e)
#[test]
fn mem_swap() {
    let mut tree_a: TreeArena<char> = TreeArena::new();
    let mut roots_a = tree_a.root_token_mut();
    roots_a.insert_child(1_u64, 'a');
    let mut node_1_a = roots_a.get_child_mut(1_u64).expect("No child 1 found");
    node_1_a.children.insert_child(2_u64, 'b');
    node_1_a.children.insert_child(3_u64, 'c');
    let mut node_3_a = node_1_a
        .children
        .get_child_mut(3_u64)
        .expect("No child 3 found");

    node_3_a.children.insert_child(4_u64, 'd');

    let mut tree_b: TreeArena<char> = TreeArena::new();
    let mut roots_b = tree_b.root_token_mut();
    roots_b.insert_child(4_u64, 'e');
    let mut node_4_b = roots_b.get_child_mut(4_u64).expect("No child 4 found");
    node_4_b.children.insert_child(3_u64, 'f');
    node_4_b.children.insert_child(2_u64, 'g');
    let mut node_2_b = node_4_b
        .children
        .get_child_mut(2_u64)
        .expect("No child 2 found");
    node_2_b.children.insert_child(1_u64, 'h');

    mem::swap(&mut node_1_a.children, &mut node_4_b.children);

    // node 1 from tree a now believes it is node 4 from tree b
    assert_eq!(node_1_a.id(), 4_u64, "Node 1 id should be 4");
    // however it still contains the item from tree a
    assert_eq!(*node_1_a.item, 'a', "Node 1 item should be 'a'");
    // and we can access the nodes in tree b, that node 4 was able to
    assert_eq!(
        *node_1_a.children.get_child(2_u64).unwrap().item,
        'g',
        "Node 2 item in tree b should be g"
    );
}
