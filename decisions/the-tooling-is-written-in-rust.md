# The tooling is written in Rust

Tags: tooling

## Rationale

The validator, scheduler, renderers and sync are one Rust codebase. Three reasons, in order of weight:

- **Schema-from-types.** The JSON Schema is generated from the validator's own serde types (`schemars`), so the schema and the validator cannot drift — exactly the co-evolution the authoring skill and the schema are meant to have. A separate schema document would be a second source of truth to keep in step by hand.
- **The scheduler deserves strong typing.** The deterministic layer (calendars, concurrency slots, critical paths) is the hardest piece; Rust's type system catches the shape errors there at compile time.
- **One binary, one ecosystem.** A static `planning-dsl` binary is the wedge CLI's cleanest distribution, and the maintainer's other project (plutus) is already a Cargo workspace — no second toolchain to maintain.

## Rejected alternatives

- TypeScript/Node: fastest schema iteration and mature ajv, but a second ecosystem beside plutus, weaker fit for the deterministic scheduler core, and npm distribution friction for a CLI wedge product.
- Python: quickest start (yaml + jsonschema), but the weakest distribution story and type safety for the product meant to harden the format.

## Evidence

Maintainer's choice in the session of 2026-11-03, on the recommendation above.
