//! The scheduler: the golden artifact for the canonical example (the daily
//! pass walked by hand in the spec), the determinism guarantee, every rule
//! the spec spells out — contention, partition, pauses, refusals — and the
//! dogfood fact that this repo's own plan refuses until it carries estimates.
//!
//! Test arithmetic is binary-exact: fractions of 0.5, weights of 1 and 3
//! (Σ 4), pools of 8, 4, 2, 6 person-hours — no float drift in the goldens.

use planning_dsl::{load, schedule, validate};

fn scheduled(yaml: &str) -> String {
    let plan = load(yaml).expect("parses");
    let report = validate(&plan);
    assert!(report.is_ok(), "fixture must validate: {:?}", report.errors);
    planning_dsl::schedule::to_yaml(&schedule(&plan).expect("schedules"))
}

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("fixtures/", $name))
    };
}

const PLAN: &str = fixture!("plan.yaml");

#[test]
fn the_canonical_example_schedules_as_the_spec_walks_it() {
    // Hand-walked: nikita (0.5) needs two 4-hour days for the 6-hour plan;
    // the dependents activate on 2026-11-12, finish-to-start; anton's
    // 40 hours span a weekend (5 working days); the review-agent's
    // `calendar: all` puts two 8-hour days on the 16-hour tests, ending
    // 2026-11-13. One chain through the 40-hour services.
    assert_eq!(
        scheduled(PLAN),
        "\
start: 2026-11-10
deadline: 2026-12-18
plan-end: 2026-11-18
slippage-days: 0
latest-start: 2026-12-10
critical-path:
- plan-out-the-tasks
- implement-services
tasks:
  design-architecture:
    status: done
  plan-out-the-tasks:
    executor: nikita
    start: 2026-11-10
    end: 2026-11-11
    active-days: 2
    status: todo
  implement-services:
    executor: anton
    start: 2026-11-12
    end: 2026-11-18
    active-days: 5
    status: todo
  implement-tests:
    executor: review-agent
    start: 2026-11-12
    end: 2026-11-13
    active-days: 2
    status: blocked
"
    );
}

#[test]
fn the_same_plan_schedules_byte_identically() {
    let plan = load(PLAN).expect("parses");
    let a = planning_dsl::schedule::to_yaml(&schedule(&plan).expect("schedules"));
    let b = planning_dsl::schedule::to_yaml(&schedule(&plan).expect("schedules"));
    assert_eq!(a, b);
}

#[test]
fn a_plan_without_a_start_is_refused() {
    let yaml = "
unit: person-hours
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton: { roles: [dev] }
";
    let plan = load(yaml).expect("parses");
    let err = schedule(&plan).expect_err("refused");
    assert!(err.contains("no start"), "{err}");
}

#[test]
fn the_estimate_gate_names_every_offender_and_exempt_done_tasks() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  a: { role: dev }
  b: { role: dev, status: blocked }
  c: { status: done, role: dev }
  d: { role: dev, estimate: 4 }
executors:
  anton: { roles: [dev] }
";
    let plan = load(yaml).expect("parses");
    let err = schedule(&plan).expect_err("refused");
    assert!(err.contains("carry no estimate"), "{err}");
    assert!(err.contains("\n  - a"), "{err}");
    assert!(err.contains("\n  - b"), "{err}");
    assert!(!err.contains("- c"), "{err}");
    assert!(!err.contains("- d"), "{err}");
}

#[test]
fn a_role_nobody_fills_is_refused_where_the_validator_warns() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
  tester: tests
tasks:
  ship: { role: dev, estimate: 8 }
  write-tests: { role: tester, estimate: 4 }
executors:
  anton: { roles: [dev] }
";
    let plan = load(yaml).expect("parses");
    assert!(validate(&plan).is_ok(), "the validator only warns");
    let err = schedule(&plan).expect_err("the scheduler refuses");
    assert!(
        err.contains("write-tests (role tester)") && err.contains("no executor fills"),
        "{err}"
    );
}

#[test]
fn capacity_that_never_returns_is_refused_naming_the_stranded() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton:
    roles: [dev]
    availability: 0.0
";
    let plan = load(yaml).expect("parses");
    let err = schedule(&plan).expect_err("refused");
    assert!(err.contains("capacity ran dry"), "{err}");
    assert!(err.contains("\n  - ship (role dev)"), "{err}");
}

#[test]
fn contention_goes_to_the_longest_downstream_path() {
    // One executor, one slot: `big` carries 12 hours downstream and wins the
    // first day over `small` (2 hours) even though `small` is declared
    // first. Declaration order breaks the true ties (`first` before `even`).
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  small: { role: dev, estimate: 2 }
  big: { role: dev, estimate: 8 }
  bigs-tail: { role: dev, estimate: 4, depends-on: [big] }
  first: { role: dev, estimate: 4 }
  even: { role: dev, estimate: 4 }
executors:
  anton:
    roles: [dev]
    calendar: all
";
    let out = scheduled(yaml);
    assert!(
        out.contains("  big:\n    executor: anton\n    start: 2026-11-02"),
        "{out}"
    );
    // Each day's full pool covers a 4-hour task, so it ends the day it
    // starts; the next in contention order takes the next day.
    assert!(
        out.contains("  bigs-tail:\n    executor: anton\n    start: 2026-11-03"),
        "{out}"
    );
    // `small` waits four days for the chain and the ties to clear.
    assert!(
        out.contains("  small:\n    executor: anton\n    start: 2026-11-06"),
        "{out}"
    );
    assert!(
        out.contains("  first:\n    executor: anton\n    start: 2026-11-04"),
        "{out}"
    );
    assert!(
        out.contains("  even:\n    executor: anton\n    start: 2026-11-05"),
        "{out}"
    );
}

#[test]
fn parallel_slots_split_the_pool_and_never_multiply_it() {
    // Four 16-hour reviews at once: the 8-hour pool splits four ways, 2
    // hours each, so every one takes 8 days — the agent calendar runs them
    // through the weekend.
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  tester: tests
tasks:
  r1: { role: tester, estimate: 16 }
  r2: { role: tester, estimate: 16 }
  r3: { role: tester, estimate: 16 }
  r4: { role: tester, estimate: 16 }
executors:
  review-agent:
    roles: [tester]
    concurrency: 4
    calendar: all
";
    let out = scheduled(yaml);
    for name in ["r1", "r2", "r3", "r4"] {
        assert!(
            out.contains(&format!("  {name}:\n    executor: review-agent\n    start: 2026-11-02\n    end: 2026-11-09\n    active-days: 8")),
            "{out}"
        );
    }
}

#[test]
fn role_weights_partition_one_executors_day_across_roles() {
    // ivan's day splits 1:3 — 2 hours to dev, 6 to tester, always. The
    // 16-hour dev task takes 8 working days, the 12-hour tester task 2,
    // side by side on the same executor.
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
  tester: tests
tasks:
  services: { role: dev, estimate: 16 }
  tests: { role: tester, estimate: 12 }
executors:
  ivan:
    role-weights: { dev: 1, tester: 3 }
    concurrency: 2
";
    let out = scheduled(yaml);
    assert!(
        out.contains("  services:\n    executor: ivan\n    start: 2026-11-02\n    end: 2026-11-11\n    active-days: 8"),
        "{out}"
    );
    assert!(
        out.contains("  tests:\n    executor: ivan\n    start: 2026-11-02\n    end: 2026-11-03\n    active-days: 2"),
        "{out}"
    );
}

#[test]
fn a_vacation_pauses_work_and_the_slot_is_held_not_reshuffled() {
    // `first` (24 h) draws 11-02 and 11-03, sits out the zero-availability
    // stretch, and finishes 11-07 — three working days. `second` is ready
    // from day one but waits behind the held slot, starting only when the
    // slot frees on 11-08.
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  first: { role: dev, estimate: 24 }
  second: { role: dev, estimate: 16 }
executors:
  anton:
    roles: [dev]
    calendar: all
    availability:
      - { from: 2026-11-04, until: 2026-11-06, fraction: 0.0 }
";
    let out = scheduled(yaml);
    assert!(
        out.contains("  first:\n    executor: anton\n    start: 2026-11-02\n    end: 2026-11-07\n    active-days: 3"),
        "{out}"
    );
    assert!(
        out.contains("  second:\n    executor: anton\n    start: 2026-11-08\n    end: 2026-11-09\n    active-days: 2"),
        "{out}"
    );
}

#[test]
fn a_dependency_ending_today_unblocks_its_dependents_tomorrow() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  build: { role: dev, estimate: 8 }
  ship: { role: dev, estimate: 8, depends-on: [build] }
executors:
  anton: { roles: [dev], calendar: all }
";
    let out = scheduled(yaml);
    assert!(
        out.contains("  build:\n    executor: anton\n    start: 2026-11-02\n    end: 2026-11-02"),
        "{out}"
    );
    assert!(
        out.contains("  ship:\n    executor: anton\n    start: 2026-11-03"),
        "{out}"
    );
}

#[test]
fn a_tiny_task_ends_the_day_it_starts() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  tick: { role: dev, estimate: 2 }
executors:
  anton: { roles: [dev], calendar: all }
";
    let out = scheduled(yaml);
    assert!(
        out.contains("  tick:\n    executor: anton\n    start: 2026-11-02\n    end: 2026-11-02\n    active-days: 1"),
        "{out}"
    );
}

#[test]
fn assignment_balances_outstanding_work_and_never_migrates() {
    // Day one: a true tie, broken by declaration order — t1 to `a`. Then
    // t2 goes to `b`, the executor with zero outstanding. On 11-04 t3 finds
    // `a` free again (t1 ended) and `b` carrying t2 — and nothing assigned
    // ever moves.
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  t1: { role: dev, estimate: 16 }
  t2: { role: dev, estimate: 16 }
  t3: { role: dev, estimate: 16, depends-on: [t1] }
executors:
  a: { roles: [dev], calendar: all }
  b: { roles: [dev], calendar: all }
";
    let out = scheduled(yaml);
    assert!(out.contains("  t1:\n    executor: a\n"), "{out}");
    assert!(out.contains("  t2:\n    executor: b\n"), "{out}");
    assert!(
        out.contains("  t3:\n    executor: a\n    start: 2026-11-04"),
        "{out}"
    );
}

#[test]
fn without_a_deadline_the_slippage_lines_are_omitted() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton: { roles: [dev], calendar: all }
";
    let out = scheduled(yaml);
    assert!(out.contains("plan-end: 2026-11-02"), "{out}");
    assert!(!out.contains("slippage"), "{out}");
    assert!(!out.contains("latest-start"), "{out}");
}

#[test]
fn a_late_plan_runs_to_completion_and_names_the_gap() {
    // The deadline has already passed by the time the one task could end;
    // the schedule is never truncated — slippage names the gap.
    let yaml = "
unit: person-hours
start: 2026-11-02
deadline: 2026-11-03
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 40 }
executors:
  anton: { roles: [dev], calendar: all }
";
    let out = scheduled(yaml);
    assert!(out.contains("plan-end: 2026-11-06"), "{out}");
    assert!(out.contains("slippage-days: 3"), "{out}");
    // The last start that still lands: deadline − the four-day span.
    assert!(out.contains("latest-start: 2026-10-30"), "{out}");
    assert!(out.contains("    end: 2026-11-06"), "{out}");
}

#[test]
fn a_plan_with_nothing_left_to_do_takes_no_dates() {
    let yaml = "
unit: person-hours
start: 2026-11-02
deadline: 2026-11-30
roles:
  dev: implements
tasks:
  seed: { status: done, role: dev }
executors:
  anton: { roles: [dev] }
";
    let out = scheduled(yaml);
    assert!(!out.contains("plan-end"), "{out}");
    assert!(!out.contains("slippage"), "{out}");
    assert!(!out.contains("latest-start"), "{out}");
    assert!(out.contains("critical-path: []"), "{out}");
    assert!(out.contains("  seed:\n    status: done\n"), "{out}");
}

#[test]
fn an_executor_carrying_both_role_shapes_is_an_input_error() {
    let yaml = "
unit: person-hours
start: 2026-11-02
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton:
    roles: [dev]
    role-weights: { dev: 1 }
";
    let plan = load(yaml).expect("parses");
    let err = schedule(&plan).expect_err("refused");
    assert!(err.contains("both `roles` and `role-weights`"), "{err}");
}

#[test]
fn the_repo_plan_schedules_its_own_remaining_work() {
    // Dogfood: the repo's plan once refused, naming `implement-scheduler` —
    // the estimate gate pointing at the task that built the gate. Now done,
    // the plan schedules: one open task, `render-gantt`, straight onto the
    // dev-agent (calendar all, 8 h/day, 6 h estimate — one day).
    let yaml = include_str!("../plan.yaml");
    let plan = load(yaml).expect("the repo plan parses");
    let out = planning_dsl::schedule::to_yaml(&schedule(&plan).expect("schedules"));
    assert!(out.contains("plan-end: 2026-11-03"), "{out}");
    assert!(
        out.contains("  render-gantt:\n    executor: dev-agent"),
        "{out}"
    );
    assert!(
        out.contains("    end: 2026-11-03\n    active-days: 1"),
        "{out}"
    );
}
