# Tree Arena

## Architecture

The unsafe tree arena contains a `DataMap` which **owns** all nodes. The `DataMap` contains:

* A `HashMap` associating `NodeId` with `Box<UnsafeCell<TreeNode<T>>>`, owning the node data, (boxed to prevent movement of the node when the `HashMap` is resized and `UnsafeCell` to express the interior mutability)

* A `HashMap` associating `NodeId` with `Option<NodeId>`, containing the parent information for the nodes

* `Box<UnsafeCell<Vec<NodeId>>>` containing the roots of the tree

It is possible to get shared (immutable) access or exclusive (mutable) access to the tree. These return `ArenaRef<'arena, T>` or `ArenaMut<'arena, T>` respectively

### Shared References

`ArenaRef<'arena, T>` contains the identity of the parent node, a reference to the node data, and `ArenaRefChildren<'arena, T>`. The `ArenaRefChildren<'arena, T>` contains the ids of the children of the node, the id of the node, and a reference to the arena. From this `ArenaRefChildren<'arena, T>` it is possible to get shared access to children of the node.

### Exclusive References

`ArenaMut<'arena, T>` contains the identity of the parent node, a mutable reference to the node data, and `ArenaMutChildren<'arena, T>`. The `ArenaMutChildren<'arena, T>` contains the ids of the children of the node, the id of the node, and a mutable reference to the arena. From this `ArenaMutChildren<'arena, T>` it is possible to get exclusive access to children of the node.

## Safety

From the `ArenaMutChildren<'arena, T>`, it is important that we can only access descendants of that node, such that we can only ever have exclusive mutable access to the contents of a node, and never have multiple mutable references. This invariant is not checked by the compiler and thus relies on the logic to determine whether a node is a descendant being correct.

## Complexity

Of finding children: $O(1)$ - previously $O(\text{children})$

Of finding deeper descendants: $O(\text{depth})$ - ideally will be made $O(1)$

Access from the root: $O(1)$, previously $O(\text{depth})$ - improved as all nodes are known to be descended from the root
