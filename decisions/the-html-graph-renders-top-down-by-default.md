# The HTML graph renders top-down by default

Tags: render, html-graph, cli

## Rationale

The interactive graph's `--direction` flag takes `tb` (top-down, the default) or `lr` (left-to-right). The direction is a property of the view, like `--gantt` is a property of the output set — not of the plan: `plan.yaml` describes work, never how a derived artifact draws it, so the flag lives on the CLI and `render::html` takes the direction as a parameter. Top-down reads the way people read plans — first things first, downward — and matches the mermaid flowchart, which is already `flowchart TD`. The direction is passed as a parameter to the renderer and on to the layout engine (`decisions/the-html-graph-is-built-on-a-vendored-standard-library.md`); only the axis changes.

## Rejected alternatives

- Keeping left-to-right as the default: a column per layer grows horizontally with the longest dependency chain, so deep plans need more scrolling than wide ones; and the accepted change is the default, not an option to hunt for.
- An in-page JS re-layout toggle (a button flipping tb/lr in the browser): the direction stays a property of the generated view — like `--gantt` — not a per-visit choice; the flag is the contract.

## Evidence

Accepted by the maintainer, 2026-11-04. Parsed in `src/main.rs` (`--direction`), honored in `render::html` (`src/render.rs`, the `Direction` enum); `tests/render.rs` pins both directions' geometry and the CLI default.
