//! Semantic validation: the checks a JSON Schema cannot express.
//!
//! The typo-class bugs die here, at commit time — `Dev` vs `dev`, a
//! misspelled dependency, a cycle, a blocked-reason on an unblocked task.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Availability, Calendar, ExecutorCalendar, Plan, Status, Weekday};

/// The estimate ceiling: one working month at the capacity model's own
/// constant, 20 working days × 8 h. Over it the task is not estimable and
/// the plan is refused (a-task-estimated-over-a-working-month-is-refused).
const MONTHLY_ESTIMATE_CEILING: f64 = 160.0;

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
    check_dates(plan, &mut r);
    check_names(plan, &mut r);
    check_calendars(plan, &mut r);
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

fn check_dates(plan: &Plan, r: &mut Report) {
    if let (Some(start), Some(deadline)) = (plan.start, plan.deadline)
        && start > deadline
    {
        r.errors.push(format!(
            "start ({start}) is after the deadline ({deadline})"
        ));
    }
}

/// The naming door: the identifying names — the mapping keys of `tasks:`,
/// `executors:` and `roles:` — are lowercase Latin or Cyrillic slugs;
/// free-form text belongs in `description:` fields, never in a decorated name
/// (identifying-names-are-lowercase-latin-or-cyrillic-slugs). Like the
/// estimate gate, it names every offender, never just the first.
fn check_names(plan: &Plan, r: &mut Report) {
    for name in plan.tasks.keys() {
        if !is_slug(name) {
            r.errors.push(format!(
                "task `{name}`: name must be a slug (lowercase Latin or Cyrillic \
                 letters and digits, hyphen-separated, starting with a letter, \
                 one script per name)"
            ));
        }
    }
    for name in plan.executors.keys() {
        if !is_slug(name) {
            r.errors.push(format!(
                "executor `{name}`: name must be a slug (lowercase Latin or Cyrillic \
                 letters and digits, hyphen-separated, starting with a letter, \
                 one script per name)"
            ));
        }
    }
    // Declared keys only: a reference to an undeclared role already dies in
    // the vocabulary check — one offender, one error, never two.
    for name in plan.roles.keys() {
        if !is_slug(name) {
            r.errors.push(format!(
                "role `{name}`: name must be a slug (lowercase Latin or Cyrillic \
                 letters and digits, hyphen-separated, starting with a letter, \
                 one script per name)"
            ));
        }
    }
}

/// The slug rule as code: `NAME_PATTERN` — starts with a lowercase letter
/// (Latin or Cyrillic, `ё` included), then lowercase letters and digits of
/// that same script, hyphens only as separators; one script per name. The
/// schema carries the same regex (`model::NAME_PATTERN`), so a name that
/// passes here passes there.
pub fn is_slug(name: &str) -> bool {
    let mut chars = name.chars();
    let cyrillic = match chars.next() {
        Some(c) if c.is_ascii_lowercase() => false,
        Some(c) if matches!(c, 'а'..='я' | 'ё') => true,
        _ => return false,
    };
    let mut after_hyphen = false;
    for c in chars {
        match c {
            '0'..='9' => after_hyphen = false,
            'a'..='z' if !cyrillic => after_hyphen = false,
            'а'..='я' | 'ё' if cyrillic => after_hyphen = false,
            '-' if !after_hyphen => after_hyphen = true,
            _ => return false,
        }
    }
    !after_hyphen
}

fn check_calendars(plan: &Plan, r: &mut Report) {
    if let Some(calendar) = &plan.calendar {
        check_calendar("calendar", calendar, r);
    }
    for (name, executor) in &plan.executors {
        match &executor.calendar {
            Some(ExecutorCalendar::All(s)) if s != "all" => {
                r.errors.push(format!(
                    "executor `{name}`: calendar must be `all` or a calendar object (found \"{s}\")"
                ));
            }
            Some(ExecutorCalendar::Of(calendar)) => {
                check_calendar(&format!("executor `{name}` calendar"), calendar, r);
            }
            _ => {}
        }
    }
}

fn check_calendar(subject: &str, calendar: &Calendar, r: &mut Report) {
    if calendar.workdays.is_empty() {
        r.errors
            .push(format!("{subject}: workdays must name at least one day"));
    }
    let mut seen = BTreeSet::new();
    for day in &calendar.workdays {
        if !seen.insert(*day) {
            r.errors
                .push(format!("{subject}: workdays repeat {}", weekday_name(*day)));
        }
    }
    let extra: BTreeSet<_> = calendar.extra_workdays.iter().copied().collect();
    for date in &calendar.holidays {
        if extra.contains(date) {
            r.errors.push(format!(
                "{subject}: {date} is both a holiday and an extra-workday"
            ));
        }
    }
}

fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
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
        if let Some(estimate) = task.estimate {
            if !estimate.is_finite() || estimate <= 0.0 {
                r.errors.push(format!(
                    "task `{name}`: estimate must be a finite number > 0 (found {estimate})"
                ));
            }
            // A working month (160 person-hours = 20 working days at 8 h) is
            // the ceiling: nothing plans a task that size, it just divides.
            // Done tasks are exempt — their estimates are records, not
            // plan inputs (a-task-estimated-over-a-working-month-is-refused).
            if task.status != Some(Status::Done) && estimate > MONTHLY_ESTIMATE_CEILING {
                r.errors.push(format!(
                    "task `{name}`: estimate {estimate} is over a working month \
                     (160 person-hours at 8 h/day); a task this size is not \
                     estimable — split it"
                ));
            }
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
    if !(0.0..=1.0).contains(&fraction) {
        r.errors.push(format!(
            "executor `{name}`: availability fraction must be in [0, 1] (found {fraction})"
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
