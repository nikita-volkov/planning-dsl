# Specs are holistic designs that code and tests project

Tags: working-system

## Rationale

A spec is the shape of a whole thing, held together; its value is exactly the whole, which is why it does not decompose into atomic truths. The layers project one another: facts constrain decisions, decisions and facts constrain specs, tasks realize specs as a `Depends:` graph, and code and tests project the accepted spec. Code contradicting its spec is a bug in the code; a spec contradicting a truth is a bug in the spec; revising a spec's sense retracts it and writes the successor, like a truth.

A spec is open while it has an *Open questions* section; code follows only accepted specs — do not keep code in step with open designs. Acceptance is the human's, after grilling; the spec is the grilling substrate.

## Rejected alternatives

- Mortal specs (distil into truths and delete on closure): destroys the wholeness that made the spec worth having; the representation model would stop reading as one design.
- The classical SDD eternal contract beside the truths as a second authority: here the truths are the binding atomic layer and the spec is the holistic layer — authority is layered, not parallel.
- Open designs living in `tasks/`: a task is an imperative granule, not a design; a design task produces a spec, it is not one.

## Evidence

Session of 2026-10-28. Retracts `code-is-generated-from-closed-truths-not-kept-in-step-with-open-ones` — its spirit (do not chase open designs with code) survives as the open-questions rule above.
