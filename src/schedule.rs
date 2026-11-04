//! The scheduler: one pure function, `validated plan.yaml → schedule
//! artifact` (`specs/scheduler.md`). Fixed rules in a fixed order, nothing
//! invented: no wall clock, no randomness, no environment — the same plan
//! produces a byte-identical schedule. TaskJuggler's semantics are the
//! reference model, minus priorities: urgency is derived from the DAG (the
//! downstream weight), never authored.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{Duration, NaiveDate};
use serde::Serialize;

use crate::model::{Availability, Calendar, Executor, Plan, Status};

/// One availability = 8 person-hours per working day. The capacity model's
/// only constant (`specs/scheduler.md`).
const DAY_HOURS: f64 = 8.0;

/// A generous bound on the daily pass: a century of calendar days. A plan
/// still mid-flight after this many simulated days cannot finish — capacity
/// that never comes back, or a fraction so small the arithmetic never
/// reaches the estimate — and is refused, not spun on.
const DAY_LIMIT: usize = 36_500;

/// The schedule artifact: everything a Gantt needs, computed, regenerable,
/// never written back into `plan.yaml`. Field order is fixed and tasks are
/// in declaration order — the serialization is part of the byte-identical
/// guarantee.
#[derive(Debug, Serialize)]
pub struct Schedule {
    pub start: NaiveDate,
    /// Present only when the plan declares a deadline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<NaiveDate>,
    /// Absent only for a plan with nothing left to do (every task done):
    /// there is no end to name.
    #[serde(rename = "plan-end", skip_serializing_if = "Option::is_none")]
    pub plan_end: Option<NaiveDate>,
    /// Days the plan end lands past the deadline; 0 when it meets it.
    /// With no deadline there is nothing to slip against — omitted.
    #[serde(rename = "slippage-days", skip_serializing_if = "Option::is_none")]
    pub slippage_days: Option<u32>,
    /// The last start date that still lands: deadline − (plan end − start).
    #[serde(rename = "latest-start", skip_serializing_if = "Option::is_none")]
    pub latest_start: Option<NaiveDate>,
    /// One chain, dependency first: from its root to the task that ends last.
    #[serde(rename = "critical-path")]
    pub critical_path: Vec<String>,
    /// Declaration order; a done task carries only its status.
    pub tasks: crate::model::DeclarationMap<TaskSchedule>,
}

/// The artifact as YAML: one fixed serialization, exactly one trailing
/// newline — the bytes the CLI prints or writes.
pub fn to_yaml(schedule: &Schedule) -> String {
    let mut out = serde_yaml_ng::to_string(schedule).expect("the artifact serializes");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The artifact as csv: the flat table, comma-separated.
pub fn to_csv(schedule: &Schedule) -> String {
    to_delimited(schedule, ',')
}

/// The artifact as tsv: the flat table, tab-separated.
pub fn to_tsv(schedule: &Schedule) -> String {
    to_delimited(schedule, '\t')
}

/// The flat table's columns, the plan block then the task block. The plan's
/// `start` is `plan-start` here, matching `plan-end`; the task block keeps
/// the artifact's `start`/`end` — no two columns share a name.
const TABLE_HEADER: [&str; 12] = [
    "plan-start",
    "deadline",
    "plan-end",
    "slippage-days",
    "latest-start",
    "task",
    "executor",
    "start",
    "end",
    "active-days",
    "status",
    "on-critical-path",
];

/// The flat-table view both delimited formats render: the plan's scalars
/// repeat on every task row and the critical-path chain becomes a per-task
/// boolean — the same data, denormalized, for spreadsheets and pipelines
/// that want rows, not a document. Column order is fixed; the serialization
/// is part of the byte-identical guarantee, same as the YAML's field order.
/// No cell is ever quoted because no cell can carry a separator: names are
/// validator-enforced slugs, the rest are dates, numbers and the status.
fn to_delimited(schedule: &Schedule, sep: char) -> String {
    let on_path: BTreeSet<&str> = schedule.critical_path.iter().map(String::as_str).collect();
    let plan_cells = [
        schedule.start.to_string(),
        cell(&schedule.deadline),
        cell(&schedule.plan_end),
        cell(&schedule.slippage_days),
        cell(&schedule.latest_start),
    ];
    let mut rows: Vec<String> = vec![line_of(&TABLE_HEADER, sep)];
    for (name, line) in &schedule.tasks {
        let mut cells = plan_cells.to_vec();
        cells.push(name.clone());
        cells.push(cell(&line.executor));
        cells.push(cell(&line.start));
        cells.push(cell(&line.end));
        cells.push(cell(&line.active_days));
        cells.push(status_cell(line.status));
        cells.push(on_path.contains(name.as_str()).to_string());
        rows.push(line_of(&cells, sep));
    }
    let mut out = rows.join("\n");
    out.push('\n');
    out
}

/// The cells of one row, separated.
fn line_of(cells: &[impl AsRef<str>], sep: char) -> String {
    let mut line = String::new();
    for (i, value) in cells.iter().enumerate() {
        if i > 0 {
            line.push(sep);
        }
        line.push_str(value.as_ref());
    }
    line
}

/// `Some` prints, `None` is the empty cell.
fn cell(value: &Option<impl std::fmt::Display>) -> String {
    value.as_ref().map_or(String::new(), |v| v.to_string())
}

/// The status in its kebab-case artifact spelling — the same string serde's
/// `rename_all` writes into the YAML.
fn status_cell(status: Status) -> String {
    match status {
        Status::Todo => "todo",
        Status::InProgress => "in-progress",
        Status::Blocked => "blocked",
        Status::Done => "done",
    }
    .to_string()
}

/// One task's line in the artifact.
#[derive(Debug, Serialize)]
pub struct TaskSchedule {
    /// Derived by role matching at activation; done tasks are never assigned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<NaiveDate>,
    /// Days the task actually drew work; paused days (vacation, holiday)
    /// draw nothing and do not count.
    #[serde(rename = "active-days", skip_serializing_if = "Option::is_none")]
    pub active_days: Option<u32>,
    /// Rides through from the plan; absent in the plan means `todo`.
    pub status: Status,
}

/// Schedule a plan. Refuses anything the spec refuses: validator errors, a
/// missing start, unestimated not-done tasks (all named), roles nobody
/// fills, capacity that can never finish the work. The error is the output
/// in those cases — the report naming the offenders is the maturity output.
pub fn schedule(plan: &Plan) -> Result<Schedule, String> {
    // The scheduler re-runs the checks and refuses on any error: it
    // consumes only plans that pass the validator.
    let report = crate::validate(plan);
    if !report.is_ok() {
        return Err(refusal(
            "the plan has validator errors; fix them first",
            &report.errors,
        ));
    }
    let start = plan.start.ok_or_else(|| {
        "error: the plan has no start: the schedule is a function of the plan and \
         a start; add `start:` to plan.yaml (a start pulled from the clock would \
         break byte-identical output)"
            .to_string()
    })?;

    let tasks: Vec<TaskEntry> = plan
        .tasks
        .iter()
        .enumerate()
        .map(|(i, (name, task))| TaskEntry {
            name,
            role: task.role.as_str(),
            estimate: task.estimate,
            deps: &task.depends_on,
            done: task.status == Some(Status::Done),
            index: i,
        })
        .collect();

    // The estimate gate: scheduling is locked until every not-done task
    // carries a positive estimate. Done tasks are exempt. Every offender is
    // named — a partial list would turn the next run into a hunt.
    let unestimated: Vec<&str> = tasks
        .iter()
        .filter(|t| !t.done && t.estimate.is_none())
        .map(|t| t.name)
        .collect();
    if !unestimated.is_empty() {
        return Err(refusal(
            "error: these not-done tasks carry no estimate; scheduling is locked \
             until every not-done task carries one",
            &unestimated,
        ));
    }

    let executors: Vec<ExecutorCapacity> = plan
        .executors
        .iter()
        .enumerate()
        .map(|(i, (name, executor))| {
            ExecutorCapacity::new(name, executor, i, plan.calendar.as_ref())
        })
        .collect::<Result<_, _>>()?;

    // A not-done task whose role no executor fills has nobody to assign; the
    // validator only warns (young plans have partial rosters), the scheduler
    // refuses — a default executor would be invented capacity.
    let unstaffed: Vec<String> = tasks
        .iter()
        .filter(|t| !t.done && !executors.iter().any(|e| e.fills(t.role)))
        .map(|t| format!("{} (role {})", t.name, t.role))
        .collect();
    if !unstaffed.is_empty() {
        return Err(refusal(
            "error: no executor fills the role of these tasks",
            &unstaffed,
        ));
    }

    // Static pass: downstream weights, one reverse-topological pass.
    let weights = downstream_weights(&tasks);

    let mut run = Run {
        cumulative: HashMap::new(),
        started: HashMap::new(),
        ended: HashMap::new(),
        assigned: HashMap::new(),
        active_days: HashMap::new(),
        slots: vec![0; executors.len()],
        tasks,
        executors,
        weights,
        start,
    };
    run.daily_pass()?;
    Ok(run.artifact(plan))
}

/// One refusal: a lead line, then every offender indented under it.
fn refusal(lead: &str, offenders: &[impl std::fmt::Display]) -> String {
    let mut out = lead.to_string();
    out.push(':');
    for offender in offenders {
        out.push_str(&format!("\n  - {offender}"));
    }
    out
}

/// A task, flattened for the run. Declaration order is the field `index`.
struct TaskEntry<'a> {
    name: &'a str,
    role: &'a str,
    estimate: Option<f64>,
    deps: &'a [String],
    done: bool,
    index: usize,
}

/// An executor's calendar, resolved once: the weekday rule, the holidays it
/// knocks out, the extra workdays it adds. `calendar: all` works every day.
struct CalendarSpec {
    /// Monday-first.
    workdays: [bool; 7],
    holidays: BTreeSet<NaiveDate>,
    extra_workdays: BTreeSet<NaiveDate>,
}

impl CalendarSpec {
    /// The executor's calendar, wholesale-replaced when authored; inherited
    /// from the plan (Monday–Friday, no holidays) when not.
    fn resolve(
        executor_calendar: Option<&crate::model::ExecutorCalendar>,
        plan_calendar: Option<&Calendar>,
    ) -> Self {
        match executor_calendar {
            Some(crate::model::ExecutorCalendar::All(_)) => CalendarSpec {
                workdays: [true; 7],
                holidays: BTreeSet::new(),
                extra_workdays: BTreeSet::new(),
            },
            Some(crate::model::ExecutorCalendar::Of(calendar)) => CalendarSpec::of(calendar),
            None => plan_calendar.map_or(
                CalendarSpec {
                    workdays: [true, true, true, true, true, false, false],
                    holidays: BTreeSet::new(),
                    extra_workdays: BTreeSet::new(),
                },
                CalendarSpec::of,
            ),
        }
    }

    fn of(calendar: &Calendar) -> Self {
        let mut workdays = [false; 7];
        for day in &calendar.workdays {
            workdays[weekday_index(*day)] = true;
        }
        CalendarSpec {
            workdays,
            holidays: calendar.holidays.iter().copied().collect(),
            extra_workdays: calendar.extra_workdays.iter().copied().collect(),
        }
    }

    fn working_day(&self, day: NaiveDate) -> bool {
        if self.extra_workdays.contains(&day) {
            return true;
        }
        if self.holidays.contains(&day) {
            return false;
        }
        self.workdays[day.weekday().num_days_from_monday() as usize]
    }
}

use chrono::Datelike;

fn weekday_index(day: crate::model::Weekday) -> usize {
    use crate::model::Weekday as W;
    match day {
        W::Mon => 0,
        W::Tue => 1,
        W::Wed => 2,
        W::Thu => 3,
        W::Fri => 4,
        W::Sat => 5,
        W::Sun => 6,
    }
}

/// An executor, precomputed for the run: calendar verdict, availability
/// lookup, role partition. Declaration order is the field `index`.
struct ExecutorCapacity<'a> {
    name: &'a str,
    /// The roles this executor can do: `roles`, or the keys of `role-weights`.
    roles: BTreeSet<&'a str>,
    /// `role-weights`: weight per role and the sum they split.
    weights: Option<(&'a BTreeMap<String, f64>, f64)>,
    calendar: CalendarSpec,
    availability: Option<&'a Availability>,
    concurrency: usize,
    index: usize,
}

impl<'a> ExecutorCapacity<'a> {
    fn new(
        name: &'a str,
        executor: &'a Executor,
        index: usize,
        plan_calendar: Option<&Calendar>,
    ) -> Result<Self, String> {
        // `roles` and `role-weights` are two shapes of the same answer; an
        // executor carries one, not both — carrying both is an input error.
        if !executor.roles.is_empty() && executor.role_weights.is_some() {
            return Err(format!(
                "error: executor `{name}` carries both `roles` and `role-weights`: \
                 two shapes of the same answer — carry one, not both"
            ));
        }
        let weights = executor.role_weights.as_ref();
        if let Some(weights) = weights {
            let mut sum = 0.0;
            for (role, weight) in weights {
                if !weight.is_finite() || *weight < 0.0 {
                    return Err(format!(
                        "error: executor `{name}`: role-weight for `{role}` must be \
                         a finite number >= 0 (found {weight})"
                    ));
                }
                sum += *weight;
            }
            if sum <= 0.0 {
                return Err(format!(
                    "error: executor `{name}`: role-weights sum to zero — no role \
                     gets any capacity"
                ));
            }
        }
        let roles: BTreeSet<&str> = match weights {
            Some(weights) => weights.keys().map(String::as_str).collect(),
            None => executor.roles.iter().map(String::as_str).collect(),
        };
        Ok(ExecutorCapacity {
            name,
            roles,
            weights: weights.map(|w| (w, w.values().sum())),
            calendar: CalendarSpec::resolve(executor.calendar.as_ref(), plan_calendar),
            availability: executor.availability.as_ref(),
            concurrency: executor.concurrency.unwrap_or(1).max(1) as usize,
            index,
        })
    }

    fn fills(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    /// The availability fraction on `day`: a scalar covers every working
    /// day; a period list overrides the stretches it names (`until`
    /// inclusive); a working day in no period is a full day. A fraction
    /// never resurrects a non-working day — the working-day check comes
    /// first in `pool`.
    fn fraction(&self, day: NaiveDate) -> f64 {
        match self.availability {
            None => 1.0,
            Some(Availability::Scalar(fraction)) => *fraction,
            Some(Availability::Periods(periods)) => periods
                .iter()
                .find(|p| p.from <= day && p.until.is_none_or(|until| until >= day))
                .map_or(1.0, |p| p.fraction),
        }
    }

    /// The pool for `role` on `day`, in person-hours: zero on a non-working
    /// day, otherwise the fraction times the day, times the role's share of
    /// the partition. With `role-weights` the pool is pre-partitioned —
    /// always; without weights the whole pool is any one role's to take.
    fn pool(&self, role: &str, day: NaiveDate) -> f64 {
        if !self.calendar.working_day(day) {
            return 0.0;
        }
        let share = match &self.weights {
            // An eligible executor fills the role, so the lookup cannot miss.
            Some((weights, sum)) => weights.get(role).unwrap_or(&0.0) / sum,
            None => 1.0,
        };
        self.fraction(day) * DAY_HOURS * share
    }

    /// Past the last authored availability date every fraction has settled
    /// to its steady value: the scalar, an open period's fraction, or a
    /// full day. The dry-capacity check looks past the horizon with it.
    fn steady_fraction(&self) -> f64 {
        match self.availability {
            None => 1.0,
            Some(Availability::Scalar(fraction)) => *fraction,
            Some(Availability::Periods(periods)) => periods
                .iter()
                .find(|p| p.until.is_none())
                .map_or(1.0, |p| p.fraction),
        }
    }
}

/// The downstream weight: the task's estimate plus the largest downstream
/// weight among its not-done dependents (sinks weigh their estimate alone;
/// done dependents contribute nothing). One reverse-topological pass,
/// Kahn's algorithm with the ready set ordered by declaration order.
fn downstream_weights(tasks: &[TaskEntry]) -> Vec<f64> {
    let mut weights: Vec<f64> = tasks
        .iter()
        .map(|t| {
            if t.done {
                0.0
            } else {
                t.estimate.unwrap_or(0.0)
            }
        })
        .collect();

    // dependents[i]: the not-done tasks that depend on tasks[i];
    // pending[i]: how many of them are still unprocessed.
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];
    let mut pending: Vec<usize> = vec![0; tasks.len()];
    for (i, task) in tasks.iter().enumerate() {
        if task.done {
            continue;
        }
        for dep in task.deps {
            // Unknown dependencies are unreachable: the validator rejects them.
            if let Some(j) = tasks
                .iter()
                .position(|t| t.name == dep.as_str())
                .filter(|&j| !tasks[j].done)
            {
                dependents[j].push(i);
                pending[j] += 1;
            }
        }
    }

    // Sinks first, in declaration order; popping a task finalizes its
    // weight and may free its dependencies.
    let mut ready: BTreeSet<usize> = (0..tasks.len())
        .filter(|&i| !tasks[i].done && pending[i] == 0)
        .collect();
    while let Some(&i) = ready.iter().next() {
        ready.remove(&i);
        let pull = dependents[i]
            .iter()
            .map(|&d| weights[d])
            .fold(0.0_f64, f64::max);
        weights[i] += pull;
        for dep_index in tasks[i]
            .deps
            .iter()
            .filter_map(|dep| tasks.iter().position(|t| t.name == dep.as_str()))
            // Done dependencies were never counted (they contribute
            // nothing); the decrement mirrors the increment.
            .filter(|&j| !tasks[j].done)
        {
            pending[dep_index] -= 1;
            if pending[dep_index] == 0 {
                ready.insert(dep_index);
            }
        }
    }
    weights
}

/// The daily pass state. All iteration is in declaration order; every
/// tie in every rule resolves through declaration order too.
struct Run<'a> {
    tasks: Vec<TaskEntry<'a>>,
    executors: Vec<ExecutorCapacity<'a>>,
    weights: Vec<f64>,
    start: NaiveDate,
    cumulative: HashMap<&'a str, f64>,
    started: HashMap<&'a str, NaiveDate>,
    ended: HashMap<&'a str, NaiveDate>,
    assigned: HashMap<&'a str, &'a str>,
    active_days: HashMap<&'a str, u32>,
    /// Occupied concurrency slots, per executor in declaration order.
    slots: Vec<usize>,
}

impl Run<'_> {
    /// Not started (never activated) and every dependency ended before
    /// `day` — finish-to-start, next-day; a done dependency counts as ended
    /// before day one.
    fn ready(&self, day: NaiveDate) -> Vec<usize> {
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                !t.done
                    && !self.started.contains_key(t.name)
                    && t.deps.iter().all(|dep| {
                        match self.tasks.iter().position(|t| t.name == dep.as_str()) {
                            Some(j) => {
                                self.tasks[j].done
                                    || self.ended.get(dep.as_str()).is_some_and(|&end| end < day)
                            }
                            // Unreachable: the validator rejects unknown deps.
                            None => true,
                        }
                    })
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn is_active(&self, i: usize) -> bool {
        let name = self.tasks[i].name;
        self.started.contains_key(name) && !self.ended.contains_key(name)
    }

    /// The sum of estimates of the executor's assigned, not-yet-ended tasks
    /// of the role — full estimates, no credit; recomputed in declaration
    /// order so the arithmetic has one fixed shape.
    fn outstanding(&self, executor: &str, role: &str) -> f64 {
        self.tasks
            .iter()
            .filter(|t| {
                !t.done
                    && t.role == role
                    && self.assigned.get(t.name).is_some_and(|a| *a == executor)
                    && !self.ended.contains_key(t.name)
            })
            .map(|t| t.estimate.unwrap_or(0.0))
            .sum()
    }

    fn daily_pass(&mut self) -> Result<(), String> {
        let mut day = self.start;
        for _ in 0..DAY_LIMIT {
            // 1. Ready, 2. Activation. Contention order: descending
            // downstream weight, then declaration order. Each ready task
            // goes to the eligible executor with the least outstanding work
            // for its role; assignment happens once, at activation — it
            // never migrates.
            let mut ready = self.ready(day);
            ready.sort_by(|&a, &b| {
                self.weights[b]
                    .total_cmp(&self.weights[a])
                    .then(self.tasks[a].index.cmp(&self.tasks[b].index))
            });
            for task_index in ready {
                let name = self.tasks[task_index].name;
                let role = self.tasks[task_index].role;
                let eligible: Vec<usize> = self
                    .executors
                    .iter()
                    .enumerate()
                    .filter(|(i, e)| {
                        e.fills(role) && self.slots[*i] < e.concurrency && e.pool(role, day) > 0.0
                    })
                    .map(|(i, _)| i)
                    .collect();
                let Some(executor_index) = eligible.iter().copied().min_by(|&a, &b| {
                    let a_name = self.executors[a].name;
                    let b_name = self.executors[b].name;
                    self.outstanding(a_name, role)
                        .total_cmp(&self.outstanding(b_name, role))
                        .then(self.executors[a].index.cmp(&self.executors[b].index))
                }) else {
                    continue; // Nobody eligible today; the task waits.
                };
                let executor = self.executors[executor_index].name;
                self.started.insert(name, day);
                self.assigned.insert(name, executor);
                self.slots[executor_index] += 1;
            }

            // 3. Draw: every active task adds its equal share of its
            // executor's role pool for today. Parallel tasks of one role
            // split that role's pool; a slot is held through zero-pool days
            // — a vacation pauses work, it does not reshuffle it.
            let active: Vec<usize> = (0..self.tasks.len())
                .filter(|&i| self.is_active(i))
                .collect();
            let mut shares: HashMap<(&str, &str), usize> = HashMap::new();
            for &i in &active {
                let task = &self.tasks[i];
                *shares
                    .entry((self.assigned[task.name], task.role))
                    .or_default() += 1;
            }
            for &i in &active {
                let task = &self.tasks[i];
                let executor = self.assigned[task.name];
                let pool = self
                    .executors
                    .iter()
                    .find(|e| e.name == executor)
                    .map_or(0.0, |e| e.pool(task.role, day));
                if pool > 0.0 {
                    let share = pool / shares[&(executor, task.role)] as f64;
                    *self.cumulative.entry(task.name).or_insert(0.0) += share;
                    *self.active_days.entry(task.name).or_insert(0) += 1;
                }
            }

            // 4. End: cumulative reaches the estimate — end today, free the
            // slot for tomorrow. Leftover capacity on a day is idle: no
            // intra-day packing.
            let mut ended_today: Vec<&str> = Vec::new();
            for &i in &active {
                let task = &self.tasks[i];
                let work = self.cumulative.get(task.name).copied().unwrap_or(0.0);
                if work >= task.estimate.unwrap_or(f64::INFINITY) {
                    self.ended.insert(task.name, day);
                    ended_today.push(task.name);
                }
            }
            for (i, executor) in self.executors.iter().enumerate() {
                self.slots[i] -= ended_today
                    .iter()
                    .filter(|name| {
                        self.assigned
                            .get(**name)
                            .is_some_and(|a| *a == executor.name)
                    })
                    .count();
            }

            if self
                .tasks
                .iter()
                .all(|t| t.done || self.ended.contains_key(t.name))
            {
                return Ok(());
            }

            // Caught, not spun on: nothing active and no capacity on any
            // later day — the work can never resume. Name the stranded.
            if active.is_empty() && !self.capacity_returns_after(day) {
                let stranded: Vec<String> = self
                    .tasks
                    .iter()
                    .filter(|t| !t.done && !self.ended.contains_key(t.name))
                    .map(|t| format!("{} (role {})", t.name, t.role))
                    .collect();
                return Err(refusal(
                    "error: capacity ran dry — no executor ever has capacity on \
                     any later day, these tasks are stranded",
                    &stranded,
                ));
            }

            day += Duration::days(1);
        }
        let unfinished: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| !t.done && !self.ended.contains_key(t.name))
            .map(|t| format!("{} (role {})", t.name, t.role))
            .collect();
        Err(refusal(
            "error: capacity ran dry — the work did not finish within the \
             simulated horizon, these tasks never accumulate their estimate",
            &unfinished,
        ))
    }

    /// Could any executor ever have a positive pool after `day`? Days up to
    /// the last authored availability date are scanned; past it every
    /// fraction has settled to its steady value, and the validator
    /// guarantees a working day exists eventually (workdays are non-empty,
    /// holidays are finite).
    fn capacity_returns_after(&self, day: NaiveDate) -> bool {
        let horizon = self
            .executors
            .iter()
            .filter_map(|e| match e.availability {
                Some(Availability::Periods(periods)) => {
                    periods.iter().map(|p| p.until.unwrap_or(p.from)).max()
                }
                _ => None,
            })
            .max();
        let Some(horizon) = horizon else {
            return self.executors.iter().any(|e| e.steady_fraction() > 0.0);
        };
        let mut scan = day + Duration::days(1);
        while scan <= horizon + Duration::days(1) {
            if self
                .executors
                .iter()
                .any(|e| e.calendar.working_day(scan) && e.fraction(scan) > 0.0)
            {
                return true;
            }
            scan += Duration::days(1);
        }
        self.executors.iter().any(|e| e.steady_fraction() > 0.0)
    }

    fn artifact(&self, plan: &Plan) -> Schedule {
        let plan_end = self
            .tasks
            .iter()
            .filter_map(|t| self.ended.get(t.name).copied())
            .max();

        // Slippage: plan end against the deadline — meets it, or late by N
        // days; a late plan is never refused and never truncated. With no
        // deadline (or nothing left to schedule) both lines are omitted.
        let (slippage_days, latest_start) = match (plan.deadline, plan_end) {
            (Some(deadline), Some(plan_end)) => {
                let span = plan_end.signed_duration_since(self.start).num_days();
                (
                    Some(plan_end.signed_duration_since(deadline).num_days().max(0) as u32),
                    Some(deadline - Duration::days(span)),
                )
            }
            _ => (None, None),
        };

        let mut tasks = crate::model::DeclarationMap::new();
        for task in &self.tasks {
            let line = if task.done {
                // Done: satisfied — occupies no capacity, unblocks its
                // dependents from day one, takes no dates, stays off the
                // critical path; the status rides through.
                TaskSchedule {
                    executor: None,
                    start: None,
                    end: None,
                    active_days: None,
                    status: Status::Done,
                }
            } else {
                TaskSchedule {
                    executor: self.assigned.get(task.name).map(|a| (*a).to_string()),
                    start: self.started.get(task.name).copied(),
                    end: self.ended.get(task.name).copied(),
                    active_days: self.active_days.get(task.name).copied(),
                    status: plan.tasks[task.name].status.unwrap_or(Status::Todo),
                }
            };
            tasks.insert(task.name.to_string(), line);
        }

        Schedule {
            start: self.start,
            deadline: plan.deadline,
            plan_end,
            slippage_days,
            latest_start,
            critical_path: self.critical_path(),
            tasks,
        }
    }

    /// One chain: start from the not-done task with the latest end (ties in
    /// declaration order), repeatedly step to its not-done dependency with
    /// the latest end (ties in declaration order), skipping done
    /// dependencies, until a task with no not-done dependencies is left.
    /// Emitted dependency-first.
    fn critical_path(&self) -> Vec<String> {
        let ends = |i: usize| self.ended.get(self.tasks[i].name).copied();
        let latest = |a: usize, b: usize| {
            ends(a)
                .unwrap_or(NaiveDate::MIN)
                .cmp(&ends(b).unwrap_or(NaiveDate::MIN))
                .then(self.tasks[a].index.cmp(&self.tasks[b].index))
        };
        let Some(head) = (0..self.tasks.len())
            .filter(|&i| !self.tasks[i].done && self.ended.contains_key(self.tasks[i].name))
            .max_by(|&a, &b| latest(a, b))
        else {
            return Vec::new();
        };
        let mut chain = vec![head];
        let mut current = head;
        while let Some(next) = (0..self.tasks.len())
            .filter(|&i| {
                !self.tasks[i].done
                    && self.tasks[current]
                        .deps
                        .iter()
                        .any(|dep| dep == self.tasks[i].name)
            })
            .max_by(|&a, &b| latest(a, b))
        {
            chain.push(next);
            current = next;
        }
        chain.reverse();
        chain
            .into_iter()
            .map(|i| self.tasks[i].name.to_string())
            .collect()
    }
}
