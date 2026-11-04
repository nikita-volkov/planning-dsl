//! The typed model of `plan.yaml`. Structure lives here and nowhere else;
//! everything computed (assignment, schedule, renders) is a separate artifact.
//!
//! Field names and shapes are the accepted spec (`specs/plan-format.md`);
//! the JSON Schema is generated from these types, so the two cannot drift.

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use chrono::NaiveDate;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A map that keeps document order: YAML mappings deserialize in file order
/// and serialize back in the same order. Declaration order is load-bearing —
/// it is the tie-break under capacity contention and the order of the
/// schedule artifact — so tasks and executors cannot live in a `BTreeMap`,
/// which would sort it away (`specs/scheduler.md`, the static pass).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclarationMap<V>(IndexMap<String, V>);

/// The shape of an identifying name: the mapping keys of `tasks:`,
/// `executors:` and `roles:` are kebab-case slugs — lowercase letters and
/// digits, hyphens only as separators, starting with a letter. The validator
/// enforces it and the schema carries this same string as `patternProperties`,
/// so the two cannot drift
/// (identifying-names-are-kebab-case-slugs).
pub const NAME_PATTERN: &str = "^[a-z][a-z0-9]*(-[a-z0-9]+)*$";

/// In the schema it is an object keyed by the slug pattern: `patternProperties`
/// built from `NAME_PATTERN` — the door the validator enforces, generated from
/// the same string — plus `additionalProperties: false`, so a key outside the
/// pattern fails schema validation too, not only validation proper.
/// `inline_schema` keeps the wrapper itself from becoming a named `$defs`
/// entry, exactly like schemars' own map impls.
impl<V: JsonSchema> JsonSchema for DeclarationMap<V> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!("map_of_{}", V::schema_name()))
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "object",
            "patternProperties": {
                (NAME_PATTERN): generator.subschema_for::<V>(),
            },
            "additionalProperties": false,
        })
    }
}

impl<V> DeclarationMap<V> {
    pub fn new() -> Self {
        DeclarationMap(IndexMap::new())
    }
}

impl<V> Default for DeclarationMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> Deref for DeclarationMap<V> {
    type Target = IndexMap<String, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for DeclarationMap<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, V> IntoIterator for &'a DeclarationMap<V> {
    type Item = (&'a String, &'a V);
    type IntoIter = indexmap::map::Iter<'a, String, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

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
    /// Declared role vocabulary; the keys are kebab-case slugs, the
    /// descriptions optional, feeding the authoring agent.
    pub roles: DeclarationMap<Option<String>>,
    pub tasks: DeclarationMap<Task>,
    pub executors: DeclarationMap<Executor>,
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
    /// Free-form prose: what the work is, for the humans and agents reading
    /// the plan. The name is the slug that identifies; the description
    /// carries what the name cannot (tasks-carry-a-free-form-description).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
