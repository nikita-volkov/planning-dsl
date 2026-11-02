# A claim lives in the smallest node that covers its scope

Tags: working-system

## Rationale

Placement walks down the tree: the claim lives in the smallest node whose subject covers its scope; when no child covers it, one level up. The root is the top such node, not a special case, so the organization is a tree all the way down. Applicability inherits downwards — a truth binds its whole subtree and never its siblings — while reading stays lazy: ancestors are read by index (`ls facts decisions autodecisions`), not by full texts.

## Rejected alternatives

- Distribution by consumer (the same venue fact filed under recording and under the model): duplicates the world; a fact has one subject and one home.
- Nesting by every distinction a `Scope:` field can express: a subfolder pays off only where facts correlate by subject; the fields remain the fine grid inside a flat folder.
