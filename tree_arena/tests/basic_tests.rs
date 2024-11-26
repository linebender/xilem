// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

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

#[test]
fn mem_swap() {
    let mut tree: TreeArena<char> = TreeArena::new();
    let mut roots = tree.root_token_mut();
    roots.insert_child(1_u64, 'a');
    let mut node_1 = roots.get_child_mut(1_u64).expect("No child 1 found");
    node_1.children.insert_child(2_u64, 'b');
    let node_1_item = node_1.item;
    let node_2 = node_1
        .children
        .get_child_mut(2_u64)
        .expect("No child 2 found");
    let node_2_item = node_2.item;
    *node_1_item = 'c';
    *node_2_item = 'd';
    #[expect(
        clippy::drop_non_drop,
        reason = "Drop glue may be added for future trees, and may differ between safe and unsafe versions"
    )]
    drop(node_2.children);
    assert_eq!(*node_1_item, 'c', "Node 1 item should be 'c'");
    assert_eq!(*node_2_item, 'd', "Node 2 item should be 'd'");
}
