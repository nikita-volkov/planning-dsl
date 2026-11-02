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
    /// Whole-project deadline — the only date a human writes.
    pub deadline: NaiveDate,
    /// Declared role vocabulary; descriptions are optional and feed the authoring agent.
    pub roles: BTreeMap<String, Option<String>>,
    pub tasks: BTreeMap<String, Task>,
    pub executors: BTreeMap<String, Executor>,
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
}

/// A calendar fraction: a scalar covers the whole project, a period list
/// covers named stretches (part-time, vacations).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Availability {
    Scalar(f64),
    Periods(Vec<Period>),
}

/// One availability stretch. `until` absent means open-ended.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Period {
    pub from: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<NaiveDate>,
    pub fraction: f64,
}
