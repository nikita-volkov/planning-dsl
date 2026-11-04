# The HTML gantt is an opt-in `render --gantt-html` flag with the mermaid gantt's gate

Tags: render, cli, schedule-artifact

## Rationale

The HTML gantt is drawn from the schedule artifact, so rendering it runs the scheduler — the same refusal-hostage situation `autodecisions/the-gantt-render-is-opt-in.md` records for the mermaid gantt: a plan that renders fine today (the flowchart and the HTML graph write for any plan that passes the validator) would start failing if the gantt were a default output, because the scheduler refuses plans without `start:`, without full estimates, with roles nobody fills, or with capacity that runs dry. Opt-in keeps the refusal where the need is: asking for the gantt html is asking for the schedule, so the refusal is the schedule's, printed unchanged, and nothing is written — the same loud gate, not a downgraded warning like the HTML graph's best-effort critical path.

On the flag's shape it mirrors `--gantt` exactly: `--gantt-html` takes an optional file name — bare `--gantt-html` writes `<stem>.gantt.html` (the `with_extension` default), `--gantt-html <file>` names it — and the flag's presence, not a missing name, is the request.

Chosen by the implementing session on 2026-11-04, without the human.

## Rejected alternatives

- **`schedule --format html`**: the schedule command's formats are flat data tables of the artifact (yaml/csv/tsv — rows a pipeline reads); an interactive view is a derived render, and `render` owns that family.
- **Folding it into `--html`**: one flag silently writing two files — the author would get a gantt they never asked for, and the always-on render would inherit the scheduler's refusal gate.
- **Format-sniffing `--gantt <file>.html`**: explicit beats implicit; the existing autodecision fixed `--gantt` as the mermaid gantt, and an extension check silently forking the output format would make the command's contract unreadable.

## Evidence

Session of 2026-11-04 implementing the HTML gantt; the opt-in shape is pinned by `the_cli_writes_the_gantt_html_only_when_asked` and the loud gate by `a_scheduler_refusal_writes_nothing_even_with_the_gantt_html_asked` in `tests/render.rs`.
