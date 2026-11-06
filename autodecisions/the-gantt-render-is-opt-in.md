# The gantt render is opt-in on `render`, because it needs a schedulable plan

Tags: render, cli, schedule-artifact

## Rationale

`planning-dsl render plan.yaml` writes the mermaid flowchart and the HTML graph for any plan that passes the validator — including young plans with no `start:` and no estimates. The gantt is different in kind: it is drawn from the schedule artifact, so rendering it runs the scheduler, which refuses a plan without `start:`, without full estimates, with roles nobody fills, or with capacity that runs dry. Making the gantt a default output of `render` would move that refusal onto every render: a plan that renders fine today would start failing, and `render`'s contract would silently fork into "any valid plan" and "schedulable plan" depending on nothing the author changed in the DAG. Opt-in keeps the refusal where the need is: asking for the gantt is asking for the schedule, so the refusal is the schedule's, printed unchanged, and nothing is written.

On the flag's shape: `--gantt-mmd` takes an optional file name — bare `--gantt-mmd` writes `<stem>.gantt.mmd` (the `with_extension` default), `--gantt-mmd <file>` names it. The gantt's absence is the opt-out, so presence-without-name is the only way the default name stays reachable: the flag's presence, not a missing name, is the request. Since 2026-11-06 all four render flags parse the same: a following non-flag token names the file, bare presence takes the default. For `--dependencies-mmd`/`--dependencies-html` a bare flag equals absence — the graph writes either way, so the request is redundant — and the parser no longer refuses it: the maintainer hit the refusal in use, reading `--dependencies-html` like `--gantt-html`, and a redundant form forgivingly accepted beats a pedantic error. This touches neither the opt-in binding nor its rejected alternatives: the gantt is still absent by default, still the scheduler's gate.

Chosen by the implementing session on 2026-11-04, without the human; the parse-shape paragraph amended 2026-11-06, also without the human, answering the same two rejected alternatives — the gantt as a default output and a separate `gantt` command remain rejected, and the fix changes only how a present flag parses, not what is written by default.

## Rejected alternatives

- **Gantt as a default `render` output**: one command exit for every render, and `render` would refuse plans it renders fine today — the mermaid and HTML outputs are hostage to the scheduler's gate for no gain.
- **A separate `gantt` command**: `render` is already the derived-outputs command and the flag shares its parse, validation and write path; a second command would duplicate the plan/validate/write plumbing for one more artifact.

## Evidence

Session of 2026-11-04 implementing the gantt granule of `tasks/render-mermaid-and-gantt-outputs.md`; the refusal path is pinned by `a_scheduler_refusal_writes_nothing_even_with_the_gantt_asked` and the opt-in shape by `the_cli_writes_the_gantt_only_when_asked` in `tests/render.rs`.
