# ifc-schedule implementation plan

Status: architecture scaffold; schedule views and temporal queries remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed work-plan, schedule, task, sequence, event, calendar, and recurrence projections.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/schedule/plan.rs`: IfcWorkPlan
- `src/schedule/work_schedule.rs`: IfcWorkSchedule
- `src/task/definition.rs`: IfcTask/type
- `src/task/time.rs`: task time variants
- `src/sequence/relation.rs`: IfcRelSequence
- `src/sequence/lag.rs`: lag values
- `src/sequence/graph.rs`: bounded DAG/cycle reporting
- `src/calendar/definition.rs`: work calendars
- `src/calendar/working_time.rs`: working periods
- `src/recurrence/pattern.rs`: recurrence patterns
- `src/recurrence/time_period.rs`: periods
- `src/event/definition.rs`: events
- `src/event/time.rs`: event time
- `src/query/timeline.rs`: deterministic temporal queries

## Work queue

- [ ] `SCHED-ROOT` - implement plans/schedules/control associations
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SCHED-TASK` - implement tasks and time variants
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SCHED-SEQ` - implement sequence/lag graph with cycle diagnostics
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SCHED-CAL` - implement calendars and recurrence expansion with budgets
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SCHED-EVENT` - implement events and event times
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SCHED-QUERY` - build deterministic timeline queries independent of cost/resources
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.
