# ifc-schedule instructions

Purpose: Borrowed work-plan, schedule, task, sequence, event, calendar, and recurrence projections.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model only; schema metadata may be added only for generic validation.

## Module ownership

- `schedule.rs`: work plans and work schedules
- `task.rs`: task identity/type/time
- `sequence.rs`: predecessor/successor and lag
- `calendar.rs`: work calendars and working times
- `recurrence.rs`: recurrence patterns and periods
- `event.rs`: events and event time
- `control.rs`: control assignments
- `query.rs`: bounded schedule graph/timeline queries
- `error.rs`: cycles and malformed temporal data

## Invariants

- Store/interpret authored schedule semantics; do not start jobs or mutate wall-clock state.
- Cycles and contradictory calendars are data errors, not recursion crashes.
- Cost/resource/product composition is application orchestration, not sibling-crate dependencies.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. New child modules remain crate-private until a real public
contract is ready for deliberate re-export.

## Verification

Run targeted tests/clippy, then the package architecture/context gates. Add
fixtures and cycle/invalid-input cases for every relationship traversal.
