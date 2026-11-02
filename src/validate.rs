//! Semantic validation: the checks a JSON Schema cannot express.
//!
//! The typo-class bugs die here, at commit time — `Dev` vs `dev`, a
//! misspelled dependency, a cycle, a blocked-reason on an unblocked task.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Availability, Plan, Status};

/// The result of validating a plan: every error, not just the first.
#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn validate(plan: &Plan) -> Report {
    let mut r = Report::default();

    check_unit(plan, &mut r);
    check_role_vocabulary(plan, &mut r);
    check_tasks(plan, &mut r);
    check_executors(plan, &mut r);
    check_cycles(plan, &mut r);
    check_assignment_coverage(plan, &mut r);

    r
}

fn check_unit(plan: &Plan, r: &mut Report) {
    if plan.unit != "person-hours" {
        r.errors.push(format!(
            "unit: only \"person-hours\" is supported (found \"{}\")",
            plan.unit
        ));
    }
}

fn check_role_vocabulary(plan: &Plan, r: &mut Report) {
    let declared: BTreeSet<&str> = plan.roles.keys().map(String::as_str).collect();
    let mut used: BTreeSet<&str> = BTreeSet::new();

    for (name, task) in &plan.tasks {
        if !declared.contains(task.role.as_str()) {
            r.errors.push(format!(
                "task `{name}`: role `{}` is not declared in roles",
                task.role
            ));
        }
        used.insert(task.role.as_str());
    }

    for (name, executor) in &plan.executors {
        for role in &executor.roles {
            if !declared.contains(role.as_str()) {
                r.errors.push(format!(
                    "executor `{name}`: role `{role}` is not declared in roles"
                ));
            }
            used.insert(role.as_str());
        }
        if let Some(weights) = &executor.role_weights {
            for role in weights.keys() {
                if !declared.contains(role.as_str()) {
                    r.errors.push(format!(
                        "executor `{name}`: role-weight role `{role}` is not declared in roles"
                    ));
                }
                used.insert(role.as_str());
            }
        }
        if executor.roles.is_empty() && executor.role_weights.is_none() {
            r.warnings.push(format!(
                "executor `{name}`: fills no roles (no `roles`, no `role-weights`)"
            ));
        }
    }

    for role in declared.difference(&used) {
        r.warnings
            .push(format!("role `{role}` is declared but never used"));
    }
}

fn check_tasks(plan: &Plan, r: &mut Report) {
    for (name, task) in &plan.tasks {
        for dep in &task.depends_on {
            if !plan.tasks.contains_key(dep) {
                r.errors
                    .push(format!("task `{name}`: depends-on `{dep}` is not a task"));
            }
        }
        if let Some(status) = task.status {
            if task.blocked_reason.is_some() && status != Status::Blocked {
                r.errors.push(format!(
                    "task `{name}`: blocked-reason is only allowed with status: blocked (status is {})",
                    status_name(status)
                ));
            }
        } else if task.blocked_reason.is_some() {
            r.errors.push(format!(
                "task `{name}`: blocked-reason is only allowed with status: blocked (status is todo)"
            ));
        }
        if let Some(estimate) = task.estimate
            && estimate <= 0.0
        {
            r.errors.push(format!(
                "task `{name}`: estimate must be > 0 (found {estimate})"
            ));
        }
    }
    if plan.tasks.is_empty() {
        r.warnings
            .push("tasks: the plan declares no tasks".to_string());
    }
}

fn check_executors(plan: &Plan, r: &mut Report) {
    for (name, executor) in &plan.executors {
        if let Some(0) = executor.concurrency {
            r.errors
                .push(format!("executor `{name}`: concurrency must be >= 1"));
        }
        match &executor.availability {
            Some(Availability::Scalar(fraction)) => {
                check_fraction(name, *fraction, r);
            }
            Some(Availability::Periods(periods)) => check_periods(name, periods, r),
            None => {}
        }
    }
}

fn check_fraction(name: &str, fraction: f64, r: &mut Report) {
    if fraction <= 0.0 || fraction > 1.0 {
        r.errors.push(format!(
            "executor `{name}`: availability fraction must be in (0, 1] (found {fraction})"
        ));
    }
}

fn check_periods(name: &str, periods: &[crate::model::Period], r: &mut Report) {
    let mut spans: BTreeMap<chrono::NaiveDate, (Option<chrono::NaiveDate>, f64)> = BTreeMap::new();
    for period in periods {
        check_fraction(name, period.fraction, r);
        if let Some(until) = period.until
            && until < period.from
        {
            r.errors.push(format!(
                "executor `{name}`: availability period ends ({until}) before it starts ({})",
                period.from
            ));
        }
        spans.insert(period.from, (period.until, period.fraction));
    }
    let ordered: Vec<_> = spans.into_iter().collect();
    for pair in ordered.windows(2) {
        let (from_a, (until_a, _)) = &pair[0];
        let (from_b, _) = &pair[1];
        let ends_a = until_a.unwrap_or(chrono::NaiveDate::MAX);
        if ends_a >= *from_b {
            r.errors.push(format!(
                "executor `{name}`: availability periods overlap (one starts {from_a}, another {from_b})"
            ));
        }
    }
}

/// Three-colour DFS over the dependency edges; reports one cycle per back-edge,
/// spelling out the path so the author can see it.
fn check_cycles(plan: &Plan, r: &mut Report) {
    #[derive(Clone, Copy, PartialEq)]
    enum Colour {
        White,
        Grey,
        Black,
    }
    let mut colour: BTreeMap<&str, Colour> = plan
        .tasks
        .keys()
        .map(|k| (k.as_str(), Colour::White))
        .collect();

    fn walk<'a>(
        task: &'a str,
        plan: &'a Plan,
        colour: &mut BTreeMap<&'a str, Colour>,
        path: &mut Vec<String>,
        r: &mut Report,
    ) {
        if colour[task] == Colour::Black {
            return;
        }
        if colour[task] == Colour::Grey {
            let start = path.iter().position(|p| p == task).unwrap_or(0);
            let cycle: Vec<String> = path[start..]
                .iter()
                .cloned()
                .chain(std::iter::once(task.to_string()))
                .collect();
            r.errors
                .push(format!("dependency cycle: {}", cycle.join(" -> ")));
            return;
        }
        colour.insert(task, Colour::Grey);
        path.push(task.to_string());
        if let Some(t) = plan.tasks.get(task) {
            for dep in &t.depends_on {
                if colour.contains_key(dep.as_str()) {
                    walk(dep, plan, colour, path, r);
                }
            }
        }
        path.pop();
        colour.insert(task, Colour::Black);
    }

    for task in plan.tasks.keys() {
        walk(task, plan, &mut colour, &mut Vec::new(), r);
    }
}

/// A task whose role nobody fills will silently never be assigned — the
/// role-vocabulary bug one level up. Warn, don't fail: young plans have
/// partial rosters.
fn check_assignment_coverage(plan: &Plan, r: &mut Report) {
    let mut filled: BTreeSet<&str> = BTreeSet::new();
    for executor in plan.executors.values() {
        for role in &executor.roles {
            filled.insert(role.as_str());
        }
        if let Some(weights) = &executor.role_weights {
            for role in weights.keys() {
                filled.insert(role.as_str());
            }
        }
    }
    for (name, task) in &plan.tasks {
        if !filled.contains(task.role.as_str()) {
            r.warnings.push(format!(
                "task `{name}`: no executor fills role `{}`; it would never be assigned",
                task.role
            ));
        }
    }
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Todo => "todo",
        Status::InProgress => "in-progress",
        Status::Blocked => "blocked",
        Status::Done => "done",
    }
}
