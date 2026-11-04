# A task estimated over a working month is refused

Tags: estimates, validator

## Rationale

A not-done task estimated over **160 person-hours** — one working month at the format's own constant, 20 working days × 8 h — is a validator error, not a warning. The maintainer's argument, 2026-11-04: a task that lasts a month makes planning meaningless — nothing interacts with it (it holds its slot through every pause, dominates the downstream-weight tie-break, and the critical path trivially runs through it), so the scheduler's dates reduce to `estimate / 8` arithmetic wearing the costume of a schedule; and a 160 h estimate is a wish, not a measurement, so every date derived from it is fake precision that reads as a plan but is not — the same lie the estimate gate exists to prevent. The validator's error class is the right home: like `estimate ≤ 0`, a magnitude that cannot serve as an estimate is never valid at any plan-maturity stage, so it dies at commit time.

Done tasks are exempt: their estimates are records of history, not plan inputs, and the estimate gate already exempts them — refusing would force the author to either delete a true record or split work that no longer exists into fiction. The honest escape hatch stays open: mark done, drop the estimate.

## Rejected alternatives

- **Validator warning** (the implementing session's original position): graded "does it run" instead of "does the output mean anything". A warning says the plan is usable with a known flaw; here the flaw voids the artifact's meaning, and a warning-buried-in-stderr is ignored exactly when it matters. Conceded by the session in the same exchange.
- **Scheduler refusal (maturity gate, like missing estimates)**: a missing estimate is an immature plan — honest, scheduling comes later; an over-month estimate is a malformed claim — no stage of maturity makes it right. The gate says "not yet"; the validator says "no".
- **An authored ceiling knob in plan.yaml**: invents config surface; the format has no knobs (single source of truth, no CLI inputs), and the 8 h/day constant is already a fixed constant of the tool. If 160 is wrong, the number changes in one commit, not per-plan.

## Evidence

Exchange of 2026-11-04: the maintainer, on the session's proposal of a validator warning — «Ну не имеет же смысла никакое планирование, если задача длится месяц. Разве нет?»; the session conceded and landed the refusal. Enforced in `src/validate.rs` (`a_not_a_month_long_task_is_enforced` in `tests/validate.rs` pins 160 passing and 161 refused); restated as a hard rule in the author-plan skill's estimate stage.
