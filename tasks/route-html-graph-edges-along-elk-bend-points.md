# Route the HTML graph's edges along ELK's bend points

Done looks like: edges in `render --html` artifacts follow ELK's computed routes — bending around nodes instead of passing over them — in both directions; arrowheads, dim/hot, crit and status styling behave as today. The vendored set is updated accordingly: if the wrapper is bypassed, `cytoscape-elk` leaves `assets/` and the autodecision records the change (new argument, not a silent rewrite). Goldens re-pinned, the two-run determinism test green, verified in a browser over `file://` offline, house demo re-rendered.

Reads: `decisions/the-html-graph-routes-edges-along-elks-bend-points.md`, `autodecisions/the-html-graph-renders-with-cytoscape-and-elk.md`, `decisions/the-html-graph-is-built-on-a-vendored-standard-library.md`.
