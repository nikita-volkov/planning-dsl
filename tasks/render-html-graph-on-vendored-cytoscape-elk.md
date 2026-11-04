# Render the HTML graph on vendored cytoscape + ELK

Done looks like: `render --html` emits one self-contained file that embeds pinned `cytoscape`, `elkjs` and `cytoscape-elk` bundles (committed under `assets/`; exact versions recorded in the autodecision) and draws the task graph with the ELK layered layout. All accepted features carry over: top-down default with the `--direction` flag, `role · estimate` labels, done muting, critical-path marks, the corner legend, pan/wheel-zoom/fit, hover neighborhood highlight, click-to-pin detail card, Shift transitive highlight, in-progress/blocked colors, arrowheads. Golden tests re-pinned; the two-run determinism test stays green. The house demo renders (`research/house-renovation-demo/house-renovation.html`, `-lr.html`) regenerated.

Starts after round 2 of the HTML graph has merged into master.

Reads: `decisions/the-html-graph-is-built-on-a-vendored-standard-library.md`, `autodecisions/the-html-graph-renders-with-cytoscape-and-elk.md`, `decisions/the-html-graph-renders-top-down-by-default.md`, `decisions/the-html-graph-labels-tasks-with-role-and-estimate.md`, `decisions/the-html-graph-mutes-done-tasks.md`, `decisions/the-html-graph-renders-the-schedules-critical-path.md`.
