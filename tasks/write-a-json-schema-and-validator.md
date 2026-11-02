Write a JSON Schema for plan.yaml and a validator CLI that checks a plan against it: declared role vocabulary (no undeclared roles anywhere), DAG well-formedness (no unknown depends-on, no cycles), enum checks, unit/deadline presence.

Done looks like: the canonical example in the spec validates clean; each typo class (`dev` vs `Dev`, unknown dependency, bad status) is rejected with a pointed message.

See: specs/plan-format.md, roles-are-a-declared-vocabulary-enforced-by-validation.

Gated on the spec's acceptance and the implementation-language question.
