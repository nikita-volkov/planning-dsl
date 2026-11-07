# planning-dsl

**One YAML file is the project plan. Dates, assignment, and the critical path are computed from it.**

`planning-dsl` is a declarative planning format for mixed teams — humans and AI agents — with a CLI over it. You declare the world: tasks with dependencies, roles, executors with their capacity, the working calendar. The tool computes the consequences: who takes which task, when it starts and ends, where the critical path runs. Deterministically — the same plan always yields the same schedule.

Editing the plan is editing a file in git. Everything else is recomputed from it.

## The pains this removes

| The pain | What planning-dsl does |
|---|---|
| Dates are typed by hand in a Gantt tool, and the plan rots the week it is written | The plan declares structure only; a deterministic scheduler computes dates, assignment and the critical path from it. Nothing computed is ever edited back into the file, so there is nothing to drift |
| Every task names a person, so the plan breaks the day someone leaves | Tasks declare roles; assignment is derived by role matching. A personnel change is one line in the executor list |
| What-if analysis means rebuilding the plan in a GUI | What-if is a file edit in git: shift `start`, add a vacation, re-run one command. Plan history is git history |
| Mixed teams — people plus AI agents — fit no planning tool | Executors are modeled uniformly: a human carries `availability` and vacations, an agent carries `concurrency` slots and a works-every-day calendar. Both are just roles plus capacity |
| Mistakes surface mid-project | The validator refuses typos, broken DAGs, wrong role names and oversized estimates at commit time — and lists every error, not just the first |
| Planning tools live in a browser tab, behind a login and a subscription | One CLI, one file format, and offline self-contained HTML renders: libraries are embedded, the file opens with no network |

## Quick start

```bash
cargo install --path .    # Rust 1.85+ (edition 2024)
```

```bash
planning-dsl validate plan.yaml                            # every error at once, not just the first
planning-dsl schedule plan.yaml                            # the schedule artifact on stdout (yaml)
planning-dsl render plan.yaml --dependencies-html --gantt-html
```

The last command writes `<stem>.dependencies.mmd`, `.dependencies.html`, `.gantt.mmd` and `.gantt.html` beside the plan — open the HTML files in a browser, they are self-contained.

## What a plan looks like

```yaml
unit: person-hours            # the estimate unit
start: 2026-11-10             # when execution begins; the scheduler refuses without it
deadline: 2026-12-18          # optional: without it, slippage is not computed

calendar:                     # absent → Mon–Fri, no holidays
  workdays: [mon, tue, wed, thu, fri]
  holidays: [2026-11-04, 2026-12-31]

roles:                        # a declared vocabulary — enforced by validation
  architect: owns system design and the RFC process
  dev: implements services
  tester: writes and maintains tests

tasks:
  design-architecture:
    status: done              # todo | in-progress | blocked | done
    role: architect
  implement-services:
    description: stand up the services the architecture RFC fixes   # prose lives here, not in names
    depends-on: [design-architecture]
    role: dev
    estimate: 40              # person-hours; over 160 the validator refuses — split the task
  implement-tests:
    depends-on: [implement-services]
    role: tester
    estimate: 16
    status: blocked
    blocked-reason: waiting on staging-environment access

executors:
  anton:
    roles: [dev]
  nikita:
    roles: [architect, dev]
    availability: 0.5         # half a day on every working day
  review-agent:               # an agent — the same executor shape
    roles: [tester]
    concurrency: 4            # four parallel slots
    calendar: all             # works every day, no holidays
```

## Commands

### `schedule` — the schedule

```bash
planning-dsl schedule plan.yaml
planning-dsl schedule plan.yaml --format csv --out schedule.csv    # or --format tsv
```

The artifact: `plan-end`, slippage against `deadline`, latest start, the critical path, and per task — executor, `start`/`end`, active days. CSV/TSV is one flat table: plan columns repeat on every task row, and the critical path becomes the `on-critical-path` column. It opens in Excel/Sheets and feeds scripts.

```yaml
plan-end: 2027-02-03
slippage-days: 0
latest-start: 2026-11-28      # the latest start that still makes the deadline
critical-path: [перегородки-второй-этаж, электрика-черновая-второй-этаж, …]
tasks:
  штукатурка-второй-этаж:
    executor: михаил
    start: 2026-12-10
    end: 2026-12-18
    active-days: 6
```

### `render` — the diagrams

| flag | what it writes | default file |
|---|---|---|
| `--dependencies-mmd` | mermaid dependency graph | `<plan>.dependencies.mmd` |
| `--dependencies-html` | interactive graph: ELK layout, role and estimate on every node, done tasks muted, critical path highlighted | `<plan>.dependencies.html` |
| `--dependencies-direction tb\|lr` | graph layout direction | `tb` |
| `--gantt-mmd` | mermaid gantt | `<plan>.gantt.mmd` |
| `--gantt-html` | interactive gantt on dhtmlx: one section per executor, critical path highlighted | `<plan>.gantt.html` |

The dependency graph always writes; the gantt is opt-in, because it runs the scheduler: the plan must have `start` and full estimates, and a refusal writes nothing. The graph is softer: on a draft that does not schedule yet it warns and writes the graph unmarked — the diagram is useful on early drafts already.

Any flag named without a file takes its default — beside the plan.

### `validate` and `emit-schema`

```bash
planning-dsl validate plan.yaml   # schema, DAG, role vocabulary, naming, the estimate ceiling
planning-dsl emit-schema          # the JSON Schema of plan.yaml — for editors and integrations
```

## What the scheduler computes

A deterministic layer over the file; TaskJuggler's semantics are the reference model.

- A task's start date comes from dependency readiness, the working calendar and executor availability.
- Executors are matched by role: humans by `availability` and `role-weights`, agents by `concurrency`.
- Capacity contention goes to the longest downstream path first — the task with more work hanging off it outranks the rest; declaration order is the stable final tie.
- The critical path, `plan-end`, slippage against `deadline`, and latest start are computed and reported in the artifact.

## The format's rules in brief

- Task, executor and role names are slugs: lowercase Latin or Cyrillic letters (`ё` included), digits, hyphens as separators, one script per name — `plan-out-the-tasks`, not `Plan Out The Tasks`. Prose goes in `description:`.
- Roles are a declared vocabulary; a `role:` must match exactly — `Dev` is not `dev`.
- One estimate per task, in person-hours. A task over 160 hours (a working month) cannot be estimated honestly — the validator refuses it and asks for decomposition. `done` tasks are exempt.
- Statuses `todo | in-progress | blocked | done` are normally written by tracker sync, not by hand: structure flows out to the tracker, status flows back.
- All dates live at plan level (`start`, `deadline` — both optional). Tasks carry no dates: the scheduler hands them out.

## A worked example: a house renovation

The demo plan is a two-story house interior renovation: a six-day work week, ten holiday days, three executors, Cyrillic task names.

```bash
planning-dsl render research/house-renovation-demo/house-renovation.yaml --dependencies-html --gantt-html
```

Ready renders sit beside the plan: [the gantt](research/house-renovation-demo/house-renovation.gantt.html), [the graph](research/house-renovation-demo/house-renovation.html), [the schedule](research/house-renovation-demo/house-renovation-schedule.yaml). Next to them — validator refusal examples with their output (`broken.yaml`, `unestimated.yaml`, `no-executor.yaml`).

## Planning by conversation

The [`.agents/skills/author-plan/`](.agents/skills/author-plan/SKILL.md) skill is for an LLM session that interviews you — scope and dates, roles, decomposition, estimates, calendar and executors — and emits a valid `plan.yaml`. Questions come with recommended options; you confirm or correct. The agent is a tool amplifying the author, not the author.

## The Theory of Constraints connection

The format is methodology-neutral, and TOC is one usage pattern over it — but the kinship is real and worth naming:

- **Contention goes to the longest downstream path.** When tasks compete for one executor's capacity, the task with more work hanging off it goes first — the same protective choice critical-chain scheduling (CCPM) makes: the chain that decides the plan's end outranks everything else. The rule stays explainable to a human team in one sentence: "your longest chain goes first."
- **The artifact already carries the quantities TOC watches.** `plan-end` against `deadline` (slippage is what a project buffer would absorb), `latest-start` (how long the chain can wait before the deadline is at risk), and the critical path (where the constraint lives) are computed on every run.
- **Buffers and bottleneck reports are the natural next layer.** Feeding buffers, bottleneck load, buffer consumption — these are derived reports over the same schedule artifact, not new formats. On the roadmap, not built yet.

## Status

v0.1.0. Present: validator, deterministic scheduler, mermaid graph and mermaid gantt, offline HTML graph and HTML gantt, CSV/TSV schedule export, JSON Schema. Next: tracker sync (Jira/Linear — structure out, status back), range estimates (PERT), TOC-flavored derived reports (buffers, bottleneck load).

## Documentation

- [`specs/plan-format.md`](specs/plan-format.md) — the authoritative format reference, with the canonical example.
- [`specs/scheduler.md`](specs/scheduler.md) — the scheduler's design and dispatch rules.
- [`decisions/`](decisions/) — why the format is what it is: every decision with its rejected alternatives.

## License

[MIT](LICENSE).
