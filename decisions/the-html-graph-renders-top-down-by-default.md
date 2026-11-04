# The HTML graph renders top-down by default

Tags: render, html-graph, cli

## Rationale

The interactive graph's `--direction` flag takes `tb` (top-down, the default) or `lr` (left-to-right). The direction is a property of the view, like `--gantt` is a property of the output set — not of the plan: `plan.yaml` describes work, never how a derived artifact draws it, so the flag lives on the CLI and `render::html` takes the direction as a parameter. Top-down reads the way people read plans — first things first, downward — and matches the mermaid flowchart, which is already `flowchart TD`. The layout stays in Rust for both directions; only the axis changes, so byte determinism is kept.

## Rejected alternatives

- Keeping left-to-right as the default: a column per layer grows horizontally with the longest dependency chain, so deep plans need more scrolling than wide ones; and the accepted change is the default, not an option to hunt for.
- An in-page JS re-layout toggle (a button flipping tb/lr in the browser): duplicates the Rust layout in JavaScript — two implementations to keep in step — and what the toggle draws would no longer be covered by the renderer's byte-determinism guarantee.

## Evidence

Accepted by the maintainer, 2026-11-04. Parsed in `src/main.rs` (`--direction`), honored in `render::html` (`src/render.rs`, the `Direction` enum); `tests/render.rs` pins both directions' geometry and the CLI default.
