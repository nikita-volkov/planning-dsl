# A module depends freely along its lineage, and across branches only sideways or up

Tags: working-system

## Rationale

A module depends on another when its truths, specs or code build on the other's: they link its truths, use its code.

**Along a lineage, dependencies are free in both directions.** A module may link its ancestors and its descendants at any depth. A parent is the natural consumer of what its children found: a claim about several subjects decomposes into per-subject facts in the children, and the parent links them ([a-claim-lives-in-the-smallest-node-that-covers-its-scope](a-claim-lives-in-the-smallest-node-that-covers-its-scope.md)). Code that wires several modules into one program lives at their common ancestor and uses them from there. A parent's links into a subtree do not travel with the subtree when it is lifted out, so they cost its self-containment nothing.

**Across branches, a module depends only on a sibling or on a sibling of one of its ancestors**, and never on a module nested inside another branch's module: that nesting is the other module's internal structure, free to change without notice. Seen from another branch, a module is its whole subtree.

**Across branches, dependencies run one way:** when A depends on B, B and its subtree do not depend on A and its subtree, directly or through other modules. Two branches depending on each other are one subject split across two folders.

Depending is not binding. A truth still binds its own subtree and never its siblings; a module that depends on a sibling takes that sibling's truths as the contract of what it uses, not as rules for its own design.

So a claim about a shared thing stays in the node of that thing, even when two siblings build on it: the shared thing becomes a module of its own, and each user depends on it. A reusable library meets a stricter test: its knowledge base reads with nothing beyond the modules it depends on, so it leans neither on domain ancestors nor on its consumers.

## Rejected alternatives

- **Only sideways or up, never down (this truth's first wording, a-module-depends-only-on-modules-at-its-own-level-or-above, retracted with this commit):** a parent could not link the facts its decisions rest on, and wiring code would have no node to live in.
- **Down to direct children only, grandchildren being a child's internals:** the parent would restate a grandchild's facts through the child instead of linking them.
- **No dependencies between siblings, with everything two siblings share lifted to their common ancestor:** a library's own design would leave the library and settle in a node whose subject is something else, often the root.
- **Dependencies on any module, nested ones in other branches included:** a module would hang on another's internal layout, and reshaping the inside of one module would break its neighbours.
- **Cycles across branches allowed:** neither branch of a cycle can be read, tested or lifted out without the other.
