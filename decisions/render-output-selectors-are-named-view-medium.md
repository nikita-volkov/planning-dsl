# Render's output selectors are named `<view>-<medium>`

Tags: render, cli, naming

## Rationale

`render` writes two views of the plan — the dependency graph and the gantt — in two media each, mermaid (`.mmd`) and interactive HTML (`.html`). The selector flags spell that grid out as `<view>-<medium>`: `--dependencies-mmd`, `--dependencies-html`, `--gantt-mmd`, `--gantt-html`. The view token comes from the format's own vocabulary — the plan's field is `depends-on`, so the graph of it is "dependencies": a name in `--help` is a word the plan's author already knows. The default file names mirror the scheme — `<stem>.<view>.<medium>` (`plan.dependencies.mmd`, `plan.dependencies.html`, `plan.gantt.mmd`, `plan.gantt.html`) — so an artifact's name says both what it is and in what medium, matching the flag that produced it. Modifiers keep plain names (`--out`, `--format`): the axis names selectors of *what to write*, not knobs of *how*. One prefixed exception the maintainer chose: `--dependencies-direction`, the one knob whose scope — only the interactive graph — was invisible from a bare name.

## Rejected alternatives

- **`dag-mmd`**: shortest and precise in form, but "dag" is graph-theory jargon absent from the format's vocabulary and the knowledge base — against the human-first positioning; nobody who writes `depends-on` says "dag".
- **`graph-mmd`**: ambiguous — the gantt is also a graph, and the name does not say what the graph is of.
- **Renaming only `--mmd` and `--gantt`**, leaving `--html`: the point of the scheme is that every selector names its view and medium; one bare flag leaves the 2×2 grid lopsided and the CLI less systematic than the other three renames alone would make it.
- **Keeping plain `--direction`**: its scope is invisible from the name and the usage text is skimmable past; the maintainer took full symmetry over brevity.
- **Keeping `<stem>.mmd`/`<stem>.html` defaults**: the always-on pair would stay out of line with the gantt pair's `<stem>.<view>.<medium>` shape — a half-applied scheme.

Old names are not aliased: a solo pre-release tool, the clean cut costs one relearned habit.

## Evidence

Session of 2026-11-05: the maintainer proposed making the flag names systematic; the session laid out the grid, the maintainer accepted — "1 ok" (dependencies), "2 ok" (mirror the defaults), "3 dependencies-direction" (the maintainer's own variant, against the session's keep-plain recommendation).
