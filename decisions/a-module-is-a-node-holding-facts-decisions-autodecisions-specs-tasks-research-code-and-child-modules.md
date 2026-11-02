# A module is a node holding facts, decisions, autodecisions, specs, tasks, research, code and child modules

Tags: working-system

## Rationale

The repository is a tree of modules. A module's folder holds everything about one subject: what we found (`facts/`), what the human chose (`decisions/`), what agent sessions chose on their own (`autodecisions/`), what we are designing (`specs/`), what we owe (`tasks/`), the raw material (`research/`), the built thing (`code/`), and its parts (`modules/`). Children sit in an explicit `modules/` container so that `ls modules/` unambiguously enumerates them and knowledge folders never collide with module names.

Cohesion is extended from code to documents: a change to a module's knowledge and the rationale behind it commute in one commit. Facts, decisions, autodecisions and specs are the durable asset; `tasks/` and `code/` are the disposable residents — tasks are deleted as they complete, code can be regenerated from closed knowledge.

## Rejected alternatives

- A flat `truths/` and `tasks/` at the root: the index outgrew readability (~165 truths of mixed scopes) while `Scope:` fields already partitioned them logically.
- Children beside knowledge folders with reserved names: one `ls` shorter, but requires every session to know the convention; the explicit container is self-describing.
- A separate repository per module: breaks the atomicity of truth-and-code commits and the single process file; extraction stays cheap because a node is self-contained by construction.
