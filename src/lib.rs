//! planning-dsl: the plan format toolchain — validator now, scheduler,
//! renderers and tracker sync later. Everything computed is a separate
//! regenerable artifact; `plan.yaml` stays pure description.

pub mod model;
pub mod validate;

pub use model::Plan;
pub use validate::{Report, validate};

/// Parse a plan from YAML. Structural errors (a bad enum value, an unknown
/// field, a missing plan-level field) die here, with serde's pointed message.
pub fn load(yaml: &str) -> Result<Plan, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("parse error: {e}"))
}

/// Validate YAML text: parse, then run the semantic checks.
pub fn check(yaml: &str) -> Result<Report, String> {
    load(yaml).map(|plan| validate(&plan))
}

/// The JSON Schema for `plan.yaml`, generated from the model types so it
/// cannot drift from the validator.
pub fn json_schema() -> serde_json::Value {
    schemars::schema_for!(Plan).into()
}
