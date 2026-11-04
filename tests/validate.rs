//! Each typo class the validator exists to kill, plus the canonical example
//! from the spec validating clean. A case fails when its pointed message
//! does not name the offending token.

use planning_dsl::check;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("fixtures/", $name))
    };
}

#[test]
fn the_canonical_example_validates_clean() {
    let report = check(fixture!("plan.yaml")).expect("parses");
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(
        report.warnings.is_empty(),
        "warnings: {:?}",
        report.warnings
    );
}

#[test]
fn the_canonical_example_round_trips() {
    let plan = planning_dsl::load(fixture!("plan.yaml")).expect("parses");
    let rewritten = serde_yaml_ng::to_string(&plan).expect("serializes");
    let reloaded = check(&rewritten).expect("parses");
    assert!(reloaded.is_ok(), "errors: {:?}", reloaded.errors);
}

#[test]
fn an_undeclared_role_is_named() {
    let report = check(fixture!("bad-role-case.yaml")).expect("parses");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("role `Dev` is not declared")),
        "{:?}",
        report.errors
    );
}

#[test]
fn an_unknown_dependency_is_named() {
    let report = check(fixture!("bad-dependency.yaml")).expect("parses");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("`design-architectur` is not a task")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_cycle_spells_out_its_path() {
    let report = check(fixture!("bad-cycle.yaml")).expect("parses");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.starts_with("dependency cycle:")
                && e.contains("implement-services")
                && e.contains("write-tests")
                && e.contains("set-up-ci")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_bad_status_dies_at_parse_with_the_variants() {
    let err = check(fixture!("bad-status.yaml")).expect_err("must not parse");
    assert!(err.contains("wip"), "{err}");
    assert!(err.contains("in-progress"), "{err}");
}

#[test]
fn a_non_person_hours_unit_is_rejected() {
    let report = check(fixture!("bad-unit.yaml")).expect("parses");
    assert!(
        report.errors.iter().any(|e| e.contains("story-points")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_blocked_reason_on_an_unblocked_task_is_rejected() {
    let report = check(fixture!("bad-blocked-reason.yaml")).expect("parses");
    assert!(
        report.errors.iter().any(|e| e.contains("blocked-reason")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_negative_estimate_is_rejected() {
    let report = check(fixture!("bad-estimate.yaml")).expect("parses");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("estimate") && e.contains("-3")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_misspelled_field_dies_at_parse() {
    let err = check(fixture!("bad-field-typo.yaml")).expect_err("must not parse");
    assert!(err.contains("depend-on"), "{err}");
}

#[test]
fn an_out_of_range_fraction_and_overlapping_periods_are_rejected() {
    let report = check(fixture!("bad-availability.yaml")).expect("parses");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("fraction") && e.contains("1.5")),
        "{:?}",
        report.errors
    );
    assert!(
        report.errors.iter().any(|e| e.contains("overlap")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_plan_without_dates_validates_clean() {
    // Both dates are optional: an uncommitted deadline is invented data,
    // worse than none.
    let report = check(fixture!("plan-without-dates.yaml")).expect("parses");
    assert!(report.is_ok(), "errors: {:?}", report.errors);
}

#[test]
fn a_start_after_the_deadline_is_rejected() {
    let yaml = "
unit: person-hours
start: 2026-12-20
deadline: 2026-12-18
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton: { roles: [dev] }
";
    let report = check(yaml).expect("parses");
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("start (2026-12-20) is after the deadline (2026-12-18)")),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_mispelled_weekday_dies_at_parse_with_the_names() {
    let yaml = "
unit: person-hours
calendar:
  workdays: [mon, mondau]
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton: { roles: [dev] }
";
    let err = check(yaml).expect_err("must not parse");
    assert!(err.contains("mondau"), "{err}");
    assert!(err.contains("mon"), "{err}");
}

#[test]
fn calendar_contradictions_are_named() {
    let yaml = "
unit: person-hours
calendar:
  workdays: [mon, tue, mon]
  holidays: [2026-11-04]
  extra-workdays: [2026-11-04]
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  anton: { roles: [dev] }
";
    let report = check(yaml).expect("parses");
    assert!(
        report.errors.iter().any(|e| e.contains("workdays repeat mon")),
        "{:?}",
        report.errors
    );
    assert!(
        report.errors.iter().any(|e| e.contains(
            "2026-11-04 is both a holiday and an extra-workday"
        )),
        "{:?}",
        report.errors
    );
}

#[test]
fn an_executor_calendar_that_is_neither_all_nor_an_object_is_rejected() {
    let yaml = "
unit: person-hours
roles:
  dev: implements
tasks:
  ship: { role: dev, estimate: 8 }
executors:
  bot:
    roles: [dev]
    calendar: everyday
";
    let report = check(yaml).expect("parses");
    assert!(
        report.errors.iter().any(|e| e.contains(
            "executor `bot`: calendar must be `all` or a calendar object (found \"everyday\")"
        )),
        "{:?}",
        report.errors
    );
}

#[test]
fn a_task_nobody_can_fill_warns() {
    let yaml = "
unit: person-hours
deadline: 2026-12-18
roles:
  dev: implements services
  tester: implements tests
tasks:
  implement-services:
    role: dev
    estimate: 40
  write-tests:
    role: tester
    estimate: 8
executors:
  anton:
    roles: [dev]
";
    let report = check(yaml).expect("parses");
    assert!(report.is_ok(), "errors: {:?}", report.errors);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("`write-tests`") && w.contains("no executor fills role `tester`")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn the_json_schema_carries_the_plan_vocabulary() {
    let schema = serde_json::to_string(&planning_dsl::json_schema()).expect("serializes");
    for token in [
        "depends-on",
        "blocked-reason",
        "role-weights",
        "availability",
        "concurrency",
        "person-hours",
        "start",
        "calendar",
        "workdays",
        "holidays",
        "extra-workdays",
    ] {
        assert!(schema.contains(token), "schema is missing `{token}`");
    }
}
