//! The typed model of `plan.yaml`. Structure lives here and nowhere else;
//! everything computed (assignment, schedule, renders) is a separate artifact.
//!
//! Field names and shapes are the accepted spec (`specs/plan-format.md`);
//! the JSON Schema is generated from these types, so the two cannot drift.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The plan: the only human-authored file. Nothing derived is ever stored here.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// Plan-level estimate unit. Only `person-hours` is valid in v1.
    pub unit: String,
    /// When execution begins. Optional here — young plans need not know —
    /// but the scheduler refuses a plan without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<NaiveDate>,
    /// Whole-project deadline, optional: an uncommitted deadline is invented
    /// data, worse than none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<NaiveDate>,
    /// Working days for the whole plan; absent → Monday–Friday, no holidays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<Calendar>,
    /// Declared role vocabulary; descriptions are optional and feed the authoring agent.
    pub roles: BTreeMap<String, Option<String>>,
    pub tasks: BTreeMap<String, Task>,
    pub executors: BTreeMap<String, Executor>,
}

/// Which days are working days: authored, never a jurisdiction string — a
/// plain read of the plan shows the whole calendar.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    /// The working weekdays; required — a calendar naming none is not one.
    pub workdays: Vec<Weekday>,
    /// Non-working dates the weekday rule would mark working (holidays).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holidays: Vec<NaiveDate>,
    /// Working dates the weekday rule would mark off (a working Saturday).
    #[serde(
        rename = "extra-workdays",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub extra_workdays: Vec<NaiveDate>,
}

/// A weekday, in the `calendar` vocabulary: `mon` … `sun`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

/// A unit of work. `estimate` is optional per task: plans mature by gaining
/// fields, and maturity is derived, never declared.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// Written by sync tooling, never hand-typed. Absent means `todo`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    pub role: String,
    #[serde(rename = "depends-on", default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate: Option<f64>,
    /// Free text; only meaningful with `status: blocked`.
    #[serde(
        rename = "blocked-reason",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub blocked_reason: Option<String>,
}

/// The four-value status enum; small enough to round-trip through any tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Todo,
    InProgress,
    Blocked,
    Done,
}

/// A human or an agent, uniformly. Humans wear `availability` (a calendar
/// fraction); agents wear `concurrency` (parallel slots). Nothing marks which
/// kind an executor is — the capacity fields they carry are the only difference.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Executor {
    /// Tracker ids, e.g. `{ jira: 3294783, linear: 82374392 }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ids: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Within-person capacity split, normalized inside this one executor.
    #[serde(
        rename = "role-weights",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub role_weights: Option<BTreeMap<String, f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub availability: Option<Availability>,
    /// Parallel slots; default 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,
    /// Replaces the plan calendar wholesale: `all` or a full calendar object.
    /// Inheritance is the default; there is no merge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<ExecutorCalendar>,
}

/// An executor-level calendar: the string `all`, or a full calendar object.
/// The untagged shape keeps the YAML honest (`calendar: all` vs
/// `calendar: { … }`); a string other than `all` is a validator error, so
/// the message can name the executor.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ExecutorCalendar {
    All(String),
    Of(Calendar),
}

/// A calendar fraction: a scalar covers every working day, a period list
/// names the stretches that differ — sparse overrides, full days elsewhere.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Availability {
    Scalar(f64),
    Periods(Vec<Period>),
}

/// One availability stretch: a fraction for a date range. `until` absent
/// means open-ended; `until` is inclusive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Period {
    pub from: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<NaiveDate>,
    pub fraction: f64,
}
