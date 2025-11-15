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

    // Force a realloc to surface potential UAFs.
    child_1.children.realloc_inner_storage();

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
    let mut node_2 = node_1.children.insert(2_u64, 'b');

    // >-- 1(a) -- 2(b)

    // Force a realloc to surface potential UAFs.
    node_2.children.realloc_inner_storage();

    let node_1_item = node_1.item;
    let node_2_item = node_2.item;
    *node_1_item = 'c';
    *node_2_item = 'd';
    *node_1_item = 'e';
    *node_2_item = 'f';
    assert_eq!(*node_1_item, 'e');
    assert_eq!(*node_2_item, 'f');
}

#[test]
fn mem_swap() {
    let mut tree_p: TreeArena<char> = TreeArena::new();
    let mut roots_p = tree_p.roots_mut();

    let mut node_p1 = roots_p.insert(1_u64, 'a');
    node_p1.children.insert(2_u64, 'b');
    let mut node_p3 = node_p1.children.insert(3_u64, 'c');
    node_p3.children.insert(4_u64, 'd');

    // P: >-- 1(a) -- 2(b)
    //             |
    //             |- 3(c) -- 4(d)

    let mut tree_q: TreeArena<char> = TreeArena::new();
    let mut roots_q = tree_q.roots_mut();

    let mut node_q4 = roots_q.insert(4_u64, 'e');
    node_q4.children.insert(3_u64, 'f');
    let mut node_q2 = node_q4.children.insert(2_u64, 'g');
    node_q2.children.insert(1_u64, 'h');

    // Q: >-- 4(e) -- 3(f)
    //             |
    //             |- 2(g)

    std::mem::swap(&mut node_p1.children, &mut node_q4.children);

    // The specifics that follow don't matter too much.
    // We mostly want to ensure this doesn't crash and MIRI doesn't detect
    // undefined behavior.

    // The node_p1 handle we've thus created still has the value 'a',
    // but now has the id '4' and access to the children of node Q4.
    assert_eq!(node_p1.id(), 4_u64);
    assert_eq!(node_p1.item, &'a');
    assert_eq!(node_p1.children.item(2_u64).unwrap().item, &'g',);
}

#[test]
fn root_ids() {
    let mut arena = TreeArena::new();
    arena.roots_mut().insert(3_u64, '0');
    arena.roots_mut().insert(4_u64, '0');
    arena.roots_mut().insert(5_u64, '0');
    assert_eq!(sorted(arena.root_ids()), vec![3, 4, 5]);
}

#[test]
fn child_ids() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(1, '1', vec![
                Node(3, '3', vec![]),
            ]),
            Node(2, '2', vec![]),
        ]),
    );
    assert_eq!(sorted(arena.find(0_u64).unwrap().child_ids()), vec![1, 2]);
    assert_eq!(sorted(arena.find(2_u64).unwrap().child_ids()), vec![]);
    assert_eq!(sorted(arena.find(3_u64).unwrap().child_ids()), vec![]);
}

#[test]
fn reparent_node_simple() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(1, '1', vec![
                Node(3, '3' ,vec![]),
            ]),
            Node(2, '2', vec![]),
        ]),
    );
    // A simple scenario where the to-be-reparented node has
    // no children and the new parent also has no children.
    arena.reparent(3_u64, 2_u64);

    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        Node(0, '0', vec![
            Node(1, '1', vec![]),
            Node(2, '2', vec![
                Node(3, '3' ,vec![]),
            ]),
        ]),
    ]);
}

#[test]
fn reparent_node_with_children() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(1, '1', vec![ // <-- to be reparented
                Node(3, '3' ,vec![]),
                Node(4, '4' ,vec![
                    Node(7, '7' ,vec![]),
                ]),
                Node(5, '5' ,vec![]),
            ]),
            Node(2, '2', vec![
                Node(6, '6' ,vec![]),
            ]),
        ]),
    );
    arena.reparent(1_u64, 2_u64);

    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        Node(0, '0', vec![
            Node(2, '2', vec![
                Node(1, '1', vec![
                    Node(3, '3' ,vec![]),
                    Node(4, '4' ,vec![
                        Node(7, '7' ,vec![]),
                    ]),
                    Node(5, '5' ,vec![]),
                ]),
                Node(6, '6' ,vec![]),
            ]),
        ]),
    ]);
}

#[test]
fn reparent_node_to_great_grandparent() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(1, '1', vec![
                Node(2, '2' ,vec![
                    Node(3, '3' ,vec![]), // <-- to be reparented
                ]),
            ]),
        ]),
    );
    arena.reparent(3_u64, 0_u64);

    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        Node(0, '0', vec![
            Node(1, '1', vec![
                Node(2, '2' ,vec![]),
            ]),
            Node(3, '3' ,vec![]),
        ]),
    ]);
}

#[test]
fn reparent_node_to_parent() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(1, '1', vec![]),
        ]),
    );
    arena.reparent(1_u64, 0_u64);

    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        // Nothing changed.
        Node(0, '0', vec![
            Node(1, '1', vec![]),
        ]),
    ]);
}

#[test]
fn reparent_between_roots() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(2, '2', vec![]),
        ]),
    );
    add_tree(&mut arena, Node(1, '1', vec![]));

    arena.reparent(2_u64, 1_u64);

    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        Node(0, '0', vec![]),
        Node(1, '1', vec![
            Node(2, '2', vec![]),
        ]),
    ]);
}

#[test]
#[should_panic(expected = "no node found for child id #1")]
fn reparent_child_not_found() {
    let mut arena = TreeArena::new();
    add_tree(&mut arena, Node(0, '0', vec![]));
    arena.reparent(1_u64, 0_u64);
}

#[test]
#[should_panic(expected = "no node found for new_parent id #2")]
fn reparent_new_parent_not_found() {
    let mut arena = TreeArena::new();
    add_tree(&mut arena, Node(0, '0', vec![Node(1, '1', vec![])]));
    arena.reparent(1_u64, 2_u64);
}

#[test]
#[should_panic(expected = "expected child to be different from new_parent but both have id #0")]
fn reparent_child_equals_new_parent() {
    let mut arena = TreeArena::<()>::new();
    arena.reparent(0_u64, 0_u64);
}

#[test]
#[should_panic(
    expected = "cannot reparent because new_parent #2 is a child of the to-be-reparented node #0"
)]
fn reparent_cycle() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(0, '0', vec![
            Node(1, '1', vec![
                Node(2, '2', vec![]),
            ]),
        ]),
    );
    arena.reparent(0_u64, 2_u64);
}

#[test]
fn reparent_root_into_other_root() {
    let mut arena = TreeArena::new();
    add_tree(&mut arena, Node(0, '0', vec![]));
    add_tree(&mut arena, Node(1, '1', vec![]));
    // Move root 0 under root 1.
    arena.reparent(0_u64, 1_u64);
    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        Node(1, '1', vec![
            Node(0, '0', vec![]),
        ]),
    ]);
}

#[test]
fn reparent_root_into_non_root() {
    let mut arena = TreeArena::new();
    #[rustfmt::skip]
    add_tree(
        &mut arena,
        Node(10, 'A', vec![
            Node(11, 'B', vec![]),
        ]),
    );
    add_tree(&mut arena, Node(20, 'C', vec![])); // another root
    // Move root 20 under node 11 (non-root).
    arena.reparent(20_u64, 11_u64);
    #[rustfmt::skip]
    assert_eq!(to_nodes(&arena), vec![
        Node(10, 'A', vec![
            Node(11, 'B', vec![
                Node(20, 'C', vec![]),
            ]),
        ]),
    ]);
}

#[derive(PartialEq, Debug)]
struct Node<T>(u64, T, Vec<Node<T>>);

fn add_tree<T>(arena: &mut TreeArena<T>, root: Node<T>) {
    let mut roots = arena.roots_mut();
    let root_mut = roots.insert(root.0, root.1);
    add_children(root.2, root_mut);
}

fn add_children<T>(children: Vec<Node<T>>, mut parent_mut: ArenaMut<'_, T>) {
    for child in children {
        let child_mut = parent_mut.children.insert(child.0, child.1);
        add_children(child.2, child_mut);
    }
}

fn to_nodes<T: Copy>(arena: &TreeArena<T>) -> Vec<Node<T>> {
    sorted(arena.root_ids())
        .into_iter()
        .map(|root_id| to_node(arena.find(root_id).unwrap()))
        .collect()
}

fn to_node<T: Copy>(a: ArenaRef<'_, T>) -> Node<T> {
    Node(
        a.id(),
        *a.item,
        sorted(a.child_ids())
            .iter()
            .map(|id| to_node(a.children.find(*id).unwrap()))
            .collect(),
    )
}

fn sorted(iter: impl IntoIterator<Item = u64>) -> Vec<u64> {
    let mut ids = Vec::from_iter(iter);
    ids.sort();
    ids
}
