# Cascading micro Style Sheets

The goal of this crate is to compute styling for a carefully curated *subset* of CSS, suitable in particular for element trees in the Xilem world, but without any dependencies (in fact, currently the only dep in the crate at all is indexmap). The goal is to be pretty compatible with browser CSS as long as you're in the subset, but not to handle arbitrary existing style sheets. In other words, it's suitable if the author controls the CSS to make it fit in the subset. Building a browser is explicitly not in scope.

The core concept is that simple CSS selectors can be evaluated as a nondeterministic finite automaton from the root to the leaf, applying tag, id, and classes (including pseudoclasses). By contrast, general CSS wants to be evaluated backward, from the leaf up to the root, and has more or less random access to the tree thanks to things like sibling combinators.

So in cuss, the entire state associated with a node is a single scalar. That references the [specified values](https://developer.mozilla.org/en-US/docs/Web/CSS/specified_value) and the NFA state.

The main operation is transitioning from a parent node to a child node. That's a sequence of NFA transitions, one each for tag, id, and each of the classes. Each such transition is recorded in a hash table. If it already exists, just get the new state. Otherwise do some work to advance the selector NFAs, and when you reach the child node, compute specified values for that (basically applying the declarations and copying over inherited values).

The fit to element trees is nearly perfect. Each node in the element tree has a `ResolveState` (which is just a newtype for a scalar). If the classes (including pseudoclasses such as hover) change, then recompute the `ResolveState` from the parent. If that's the same as before, you're done. Otherwise, diff the values and apply those to rendering or layout, and traverse to your children.

Thus, cuss is not just CSS style resolution, it is potentially a very efficient *incremental* algorithm.

Current state: still a rough protoype with a number of missing features, and no work done yet to integrate with other systems. Even so, many of the core ideas are in place.
