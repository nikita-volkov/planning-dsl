# The HTML graph routes its edges along ELK's bend points

Tags: render, html-graph

## Rationale

The vendored ELK layered layout computes full edge routes — start point, bends around nodes, end point — but the `cytoscape-elk` wrapper applies only node positions and lets cytoscape draw straight lines, so edges pass over nodes: the visible defect of the previous renderer, partially reintroduced. The maintainer accepted routing the edges along ELK's computed sections (2026-11-06): first check the wrapper for an option that preserves them — none is known — so the glue runs the vendored ELK directly and feeds its edge sections to cytoscape as `curve-style: 'segments'` segment points. The engine stays the vendored ELK; this is plumbing of its output into the renderer, not our own layout algorithm.

## Rejected alternatives

- Accepting straight edges: crossing minimization already lands and layering is ELK's, but the edges-over-nodes defect partially returns — the trade the maintainer declined.

## Evidence

Maintainer's acceptance («1 ок») in session of 2026-11-06, on the recommendation given 2026-11-04. The deviation was found while implementing 4279e96: `cytoscape-elk` 2.3.0 applies node positions only (recorded in `autodecisions/the-html-graph-renders-with-cytoscape-and-elk.md`).
