# The HTML graph renders with vendored cytoscape and the ELK layered layout

Tags: render, html-graph

## Rationale

Cytoscape (MIT) is the standard interactive graph *viewer*: a canvas renderer with pan, wheel zoom to cursor, hover and tap events, touch support, fit-to-view, neighborhood/successor/predecessor graph APIs and class-based styling — every interaction the artifact needs (hover highlight, click-to-pin card, Shift transitive highlight, critical-path and status styling) is configuration, not code. Its DAG layout comes from an extension: elkjs (EPL-2.0) wrapping the Eclipse Layout Kernel's layered algorithm, the strongest layered layout in the landscape — proper crossing minimization and orthogonal edge routing around nodes, which is precisely the quality gap of the hand-rolled renderer (longest-path layering, declaration order, straight edges).

Vendoring: the dist bundles (`cytoscape.min.js`, `elk.bundled.js`, the `cytoscape-elk` wrapper) are pinned to exact versions, committed to the repo, and embedded verbatim into the generated file; license headers stay intact and an attribution note names the libraries. The artifact remains one self-contained file that opens offline. Known accepted costs: the generated artifact grows to roughly 2 MB (the ELK engine is a GWT-compiled blob) and labels are canvas-rendered, so task names are not browser-findable (Ctrl+F).

## Rejected alternatives

- **Cytoscape + dagre** (`cytoscape-dagre`): a ~0.5 MB artifact instead of ~2 MB, but dagre lacks routing around nodes — edges graze or cross them in dense graphs, and edge quality is the gap being fixed.
- **Dagre alone, keeping the hand-rolled SVG viewer**: vendors only coordinates while our own viewer glue stays — the maintainer said not to write our own; cytoscape also brings touch support and the successor/predecessor APIs the glue lacks.
- **Graphviz in the browser** (`@viz-js/viz`): dot-quality layout, but the static SVG it emits leaves the whole interaction layer to be built again on top; same ~2 MB order of size.
- **React Flow**: the current standard for node editors, but it needs React and a bundler — foreign to a no-build, single-file, offline artifact.
- **Mermaid flowchart**: renders on a dagre-family engine, but pan/zoom, click details and highlighting are not products of the library.
- **maxgraph / jointjs**: editor frameworks, heavier than a read-only viewer needs.

## Evidence

npm pages fetched 2026-11-04: cytoscape 3.34.4, 25,339,772 weekly downloads, MIT; elkjs 0.12.1, 17,872,832 weekly, EPL-2.0/GPL-3.0; @dagrejs/dagre 3.1.3, 7,870,643 weekly, MIT (its skeleton is Gansner et al. 1993 per its wiki); @viz-js/viz 3.31.0, MIT. Chosen by the session on 2026-11-04 under `decisions/the-html-graph-is-built-on-a-vendored-standard-library.md`.

Implemented 2026-11-04 (session html-cytoscape): the pinned versions as vendored under `assets/` are cytoscape **3.34.4** (`cytoscape-3.34.4.min.js`, sha256 081591ab…), elkjs **0.12.1** (`elk-0.12.1.bundled.js`, sha256 b1032b7c…) and cytoscape-elk **2.3.0** — its latest published version at fetch time, peer `cytoscape ^3.2.0` satisfied (`cytoscape-elk-2.3.0.js`, sha256 25acbb9d…) — tarballs pulled from registry.npmjs.org with their sha512 integrity verified against the registry. The wrapper resolves the ELK engine from the `ELK` global (its production build externalizes elkjs), so the load order is cytoscape → elk.bundled → cytoscape-elk; `new ELK()` with no worker options runs ELK's embedded main-thread fallback, so the artifact opens offline from `file://`. The fixture artifact measures 2,104,537 bytes — the accepted ~2 MB cost, on the nose. The wrapper applies ELK's node positions only (its adapter drops ELK's bend points), so edges draw straight; layering, ordering and crossing minimization are ELK's.
