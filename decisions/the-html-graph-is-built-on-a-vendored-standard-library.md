# The HTML graph is built on a vendored standard JavaScript library

Tags: render, html-graph

## Rationale

The interactive graph's layout and interaction machinery are a standard library's job, not ours. The maintainer struck down the layout-in-Rust stance on 2026-11-04: vendoring JavaScript libraries is fine, and the graph should use the best ready-made implementation with the best UX — nothing hand-rolled beyond configuration. The artifact keeps its accepted contract — one self-contained file, no CDN, no network, opens offline — by vendoring: the library's dist bundles are pinned versions committed to the repo and embedded verbatim into the generated file, license headers intact. Byte determinism survives where it matters: the same plan plus the same vendored blobs produce the same artifact bytes; the rendered geometry is the library's, deterministic for a pinned version.

## Rejected alternatives

- A native Rust port of the dot algorithm (weighted median + transpose ordering, Brandes–Köpf coordinates) — the previous session's proposal: it keeps layout in Rust but writes our own implementation of a solved problem, exactly what the maintainer rejected. The standard method is worth adopting only through a standard implementation.
- Keeping layout in Rust with hand-rolled geometry for byte-determinism's sake: the rationale was struck down by the maintainer — byte determinism of artifact bytes does not require owning the layout; and the hand-rolled layout is the one part of the artifact below the standards' bar (no crossing minimization, no edge routing around nodes).

## Evidence

Maintainer's acceptance in session of 2026-11-04: «Бредовое решение какое-то. Я не вижу проблем с вендорингом javascript библиотек. Давай выберем наилучшее решение с лучшим UX. Нет необходимости писать своё.» Research digest of the same session: dagre/elkjs/cytoscape landscape with versions and downloads.
