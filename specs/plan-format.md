# The plan format

Tags: plan-format

A planning DSL for mixed human/agent teams. `plan.yaml` — the only human-authored file — declares tasks as a dependency DAG with roles, and executors (humans and AI agents uniformly) with roles, capacity and tracker ids. Everything else is derived: a deterministic scheduler computes timing from estimates and capacity; renderers emit mermaid, an interactive HTML graph and a Gantt chart; exporters write Jira/Linear import formats; status flows back from trackers. The file is authored and refined through LLM grilling (structured interview) sessions.

The design was settled in three grilling rounds on 2026-11-02; its twelve decisions live in `decisions/` and are linked below. This spec is the whole held together. **Accepted by the maintainer on 2026-11-03**; the tooling language is settled in [the-tooling-is-written-in-rust](../decisions/the-tooling-is-written-in-rust.md). The scheduler's tie-break rule under capacity contention is the one design point deliberately left to the scheduler's own spec (task: [settle-the-capacity-contention-tie-break](../tasks/settle-the-capacity-contention-tie-break.md)).

## The canonical example

```yaml
# plan.yaml — the only human-authored file.
# Structure lives here; status flows back from trackers; nothing computed.
unit: person-hours            # plan-level estimate unit
deadline: 2026-12-18          # whole-project deadline (the only date in the file)

roles:                        # declared vocabulary — validated, typo-proof
  architect: owns system design and the RFC process
  planner: decomposes work into estimable tasks
  dev: implements services
  tester: implements and maintains tests

tasks:
  design-architecture:
    status: done              # todo | in-progress | blocked | done
    role: architect
  plan-out-the-tasks:
    depends-on: [design-architecture]
    role: planner
    estimate: 6
  implement-services:
    depends-on: [plan-out-the-tasks]
    role: dev
    estimate: 40
  implement-tests:
    depends-on: [plan-out-the-tasks]
    role: tester
    estimate: 16
    status: blocked
    blocked-reason: waiting on staging-environment budget approval

executors:
  anton:
    ids: { jira: 3294783, linear: 82374392 }
    roles: [dev]
  nikita:
    roles: [architect, planner]
    availability: 0.5                  # scalar = one period, whole project
  ivan:
    role-weights: { dev: 2, tester: 7 }  # within-person capacity split (2/9 dev)
    availability:                      # period-list form
      - { from: 2026-11-01, until: 2026-12-15, fraction: 0.5 }
      - { from: 2026-12-16, fraction: 1.0 }
  review-agent:
    roles: [tester]
    concurrency: 4                     # 4 parallel slots; no calendar needed
```

## The invariant: authored vs. derived

- **Authored in `plan.yaml`:** `unit`, `deadline`, the `roles` vocabulary; tasks (`role`, `depends-on`, `estimate`, `status`, `blocked-reason`); executors (`ids`, `roles`/`role-weights`, `availability`, `concurrency`).
- **Derived, disposable, never written back:** assignment, maturity stage, schedule (start/end), critical path, mermaid/HTML/Gantt renders, tracker exports.

`status`/`blocked-reason` sit in the file but are written by sync tooling, never hand-typed: structure flows out to trackers, status flows back.

## Layers

- **Validator** — checks the declared role vocabulary, the DAG, the schema. Typo-class bugs die at commit time.
- **Scheduler** — deterministic layer over the file: consumes estimates, availability periods, concurrency slots and the plan deadline; produces start/end dates and the critical path. TaskJuggler's semantics are the reference model ([taskjuggler-solves-deterministic-scheduling-and-has-been-dormant-since-2020](../facts/taskjuggler-solves-deterministic-scheduling-and-has-been-dormant-since-2020.md)).
- **Renderers** — mermaid graph, interactive HTML graph, Gantt chart.
- **Tracker sync** — Jira/Linear exports out, status back, asymmetric authority.
- **Authoring skill** — the grilling-session skill that interviews a human and emits/refines `plan.yaml`; the wedge use case, and the force that hardens the schema against real plans.

## Positioning

Human-team planner: open format, agents-as-tools. The wedge is grilling session → plan file → tracker export. Agent orchestration (the same format used to schedule subagent work) is a consumer, not the positioning.

## Decisions this spec holds together

- [the-format-is-descriptive-and-the-scheduler-is-a-deterministic-layer-over-it](../decisions/the-format-is-descriptive-and-the-scheduler-is-a-deterministic-layer-over-it.md)
- [estimates-are-single-person-hours-declared-once-at-plan-level](../decisions/estimates-are-single-person-hours-declared-once-at-plan-level.md)
- [maturity-is-derived-from-estimates-never-declared](../decisions/maturity-is-derived-from-estimates-never-declared.md)
- [assignment-is-derived-by-role-matching-and-the-plan-stays-role-only](../decisions/assignment-is-derived-by-role-matching-and-the-plan-stays-role-only.md)
- [role-weights-split-one-executors-capacity-across-roles](../decisions/role-weights-split-one-executors-capacity-across-roles.md)
- [roles-are-a-declared-vocabulary-enforced-by-validation](../decisions/roles-are-a-declared-vocabulary-enforced-by-validation.md)
- [status-is-a-four-value-enum-and-blocked-reason-is-written-by-tooling](../decisions/status-is-a-four-value-enum-and-blocked-reason-is-written-by-tooling.md)
- [sync-is-asymmetric-structure-flows-out-and-status-flows-back](../decisions/sync-is-asymmetric-structure-flows-out-and-status-flows-back.md)
- [executors-are-humans-and-agents-with-availability-and-concurrency](../decisions/executors-are-humans-and-agents-with-availability-and-concurrency.md)
- [the-deadline-is-plan-level-only](../decisions/the-deadline-is-plan-level-only.md)
- [computed-outputs-are-separate-regenerable-artifacts](../decisions/computed-outputs-are-separate-regenerable-artifacts.md)
- [the-positioning-is-a-human-team-planner-with-agents-as-tools](../decisions/the-positioning-is-a-human-team-planner-with-agents-as-tools.md)
- [the-tooling-is-written-in-rust](../decisions/the-tooling-is-written-in-rust.md)
