<div align="center">

# Tree Arena

**A tree implementation for Masonry**

[![Latest published version.](https://img.shields.io/crates/v/tree_arena.svg)](https://crates.io/crates/tree_arena)
[![Documentation build status.](https://img.shields.io/docsrs/tree_arena.svg)](https://docs.rs/tree_arena)
[![Apache 2.0 license.](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23masonry-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/stream/317477-masonry)
[![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/xilem/ci.yml?logo=github&label=CI)](https://github.com/linebender/xilem/actions)
[![Dependency staleness status.](https://deps.rs/crate/tree_arena/latest/status.svg)](https://deps.rs/crate/tree_arena)

</div>

<!-- We use cargo-rdme to update the README with the contents of lib.rs.
To edit the following section, update it in lib.rs, then run:
cargo rdme --workspace-project=tree_arena --heading-base-level=0
Full documentation at https://github.com/orium/cargo-rdme -->

<!-- Intra-doc links used in lib.rs should be evaluated here.
See https://linebender.org/blog/doc-include/ for related discussion. -->

<!-- cargo-rdme start -->

This crate contains two implementations of a tree for use in [Masonry], one safe and the other unsafe. The safe tree is known to work, and serves as the baseline implementation and is used by default.
The unsafe tree leverages a hashmap as an arena and is designed for higher performance: it leverages unsafe code to achieve this. The unsafe tree is not yet fully tested, and is not used by default.

The safe tree is the priority. This means:

* The safe version may have features / APIs that the unsafe version doesn't yet have.

* If both versions are at feature parity, [Masonry] can switch on the unsafe version for best performance.

* Otherwise, [Masonry] uses the safe version.

## Architecture

### Safe Tree

The safe tree contains a root `TreeArena` which owns the root nodes as `Vec<TreeNode<T>>`, and a`parents_map` tracking the parent of every node.
Each `TreeNode` subsequently owns its own children as `Vec<TreeNode<T>>`. This model of owneship is thus checked by the rust compiler,
but has the downside of requiring passing through every ancestor node to access the descendant -
this requires an O(depth) determination of whether the node is a descendant, followed by O(children) time at each level to traverse the path to the child.

### Unsafe Tree

The unsafe tree arena contains a `DataMap` which **owns** all nodes. The `DataMap` contains:

* A `HashMap` associating `NodeId` with `Box<UnsafeCell<TreeNode<T>>>`, owning the node data, (boxed to prevent movement of the node when the `HashMap` is resized and `UnsafeCell` to express the interior mutability)

* A `HashMap` associating `NodeId` with `Option<NodeId>`, containing the parent information for the nodes

* `Box<UnsafeCell<Vec<NodeId>>>` containing the roots of the tree

It is possible to get shared (immutable) access or exclusive (mutable) access to the tree. These return `ArenaRef<'arena, T>` or `ArenaMut<'arena, T>` respectively.
We do this by leveraging a hash map to store the nodes: from this we can obtain either shared or exclusive access to nodes.
To ensure that only one item is allowed to create new exclusive access to nodes, this action requires mutable access to the arena as a whole (and so is checked by the compiler) -
what the compiler cannot check is that the nodes accessed mutably are distinct from one another - this is done by only allowing access to descendants of the node being accessed mutably.
The aim of this is to reduce the time needed to access node, as given a node, we only need to determine whether it is a descendant of the node being accessed mutably,
and do not need to iterate over the children and to flatten the overall tree graph into a hash map.

#### Shared References

`ArenaRef<'arena, T>` contains the identity of the parent node, a reference to the node data, and `ArenaRefChildren<'arena, T>`.
The `ArenaRefChildren<'arena, T>` contains the ids of the children of the node, the id of the node, and a reference to the arena. From this `ArenaRefChildren<'arena, T>` it is possible to get shared access to children of the node.

#### Exclusive References

`ArenaMut<'arena, T>` contains the identity of the parent node, a mutable reference to the node data, and `ArenaMutChildren<'arena, T>`.
The `ArenaMutChildren<'arena, T>` contains the ids of the children of the node, the id of the node, and a mutable reference to the arena.
From this `ArenaMutChildren<'arena, T>` it is possible to get exclusive access to children of the node.

#### Safety

From the `ArenaMutChildren<'arena, T>`, it is important that we can only access descendants of that node,
such that we can only ever have exclusive mutable access to the contents of a node, and never have multiple mutable references.
This invariant is not checked by the compiler and thus relies on the logic to determine whether a node is a descendant being correct.

### Complexity

|Operation  | Safe         | Unsafe   |
|   ---     |      ---     |   ---    |
|Find child | O(Children)  | O(1)     |
|Descendant | O(Depth)     | O(Depth) |
|From root  | O(Depth)     | O(1)     |

[Masonry]: https://crates.io/crates/masonry

<!-- cargo-rdme end -->

## Minimum supported Rust Version (MSRV)

This version of Tree Arena has been verified to compile with **Rust 1.86** and later.

Future versions of Tree Arena might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

<details>
<summary>Click here if compiling fails.</summary>

As time has passed, some of Tree Arena's dependencies could have released versions with a higher Rust requirement.
If you encounter a compilation issue due to a dependency and don't want to upgrade your Rust toolchain, then you could downgrade the dependency.

```sh
# Use the problematic dependency's name and version
cargo update -p package_name --precise 0.1.1
```
</details>

## Community

Discussion of Tree Arena development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#masonry channel](https://xi.zulipchat.com/#narrow/stream/317477-masonry).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
