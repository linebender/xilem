# Tree Arena

This crate contains two implementations of a tree for use in masonry, one safe and the other unsafe. The safe tree is known to work, and serves a s the baseline implementation and is used by default. The unsafe tree leverages a hashmap as an arena and is designed for higher performance: it leverages unsafe code to achieve this. The unsafe tree is not yet fully tested, and is not used by default.

## Architecture

### Safe Tree

The safe tree contains a root `TreeArena` which owns the root nodes as `Vec<TreeNode<T>>`, and a`parents_map` tracking the parent of every node. Each `TreeNode` subsequently owns its own children as `Vec<TreeNode<T>>`. This model of owneship is thus checked by the rust compiler, but has the downside of requiring passing through every ancestor node to access the descendant - this requires an O(depth) determination of whether the node is a descendant, followed by O(children) time at each level to traverse the path to the child.

### Unsafe Tree

The unsafe tree arena contains a `DataMap` which **owns** all nodes. The `DataMap` contains:

* A `HashMap` associating `NodeId` with `Box<UnsafeCell<TreeNode<T>>>`, owning the node data, (boxed to prevent movement of the node when the `HashMap` is resized and `UnsafeCell` to express the interior mutability)

* A `HashMap` associating `NodeId` with `Option<NodeId>`, containing the parent information for the nodes

* `Box<UnsafeCell<Vec<NodeId>>>` containing the roots of the tree

It is possible to get shared (immutable) access or exclusive (mutable) access to the tree. These return `ArenaRef<'arena, T>` or `ArenaMut<'arena, T>` respectively. We do this by leveraging a hash map to store the nodes: from this we can obtain either shared or exclusive access to nodes. To ensure that only one item is allowed to create new exclusive access to nodes, this action requires mutable access to the arena as a whole (and so is checked by the compiler) - what the compiler cannot check is that the nodes accessed mutably are distinct from one another - this is done by only allowing access to descendants of the node being accessed mutably. The aim of this is to reduce the time needed to access node, as given a node, we only need to determine whether it is a descendant of the node being accessed mutably, and do not need to iterate over the children and to flatten the overall tree graph into a hash map.

#### Shared References

`ArenaRef<'arena, T>` contains the identity of the parent node, a reference to the node data, and `ArenaRefChildren<'arena, T>`. The `ArenaRefChildren<'arena, T>` contains the ids of the children of the node, the id of the node, and a reference to the arena. From this `ArenaRefChildren<'arena, T>` it is possible to get shared access to children of the node.

#### Exclusive References

`ArenaMut<'arena, T>` contains the identity of the parent node, a mutable reference to the node data, and `ArenaMutChildren<'arena, T>`. The `ArenaMutChildren<'arena, T>` contains the ids of the children of the node, the id of the node, and a mutable reference to the arena. From this `ArenaMutChildren<'arena, T>` it is possible to get exclusive access to children of the node.

#### Safety

From the `ArenaMutChildren<'arena, T>`, it is important that we can only access descendants of that node, such that we can only ever have exclusive mutable access to the contents of a node, and never have multiple mutable references. This invariant is not checked by the compiler and thus relies on the logic to determine whether a node is a descendant being correct.

### Complexity

|Operation  | Safe         | Unsafe   |
|   ---     |      ---     |   ---    |
|Find child | O(Children)  | O(1)     |
|Descendant | O(Depth)     | O(Depth) |
|From root  | O(Depth)     | O(1)     |
