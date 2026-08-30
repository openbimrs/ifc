# ifc-schedule implementation plan

Status: implemented; every task in the work queue is complete.
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

- [x] `SCHED-ROOT` - implement plans/schedules/control associations
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `SCHED-TASK` - implement tasks and time variants
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `SCHED-SEQ` - implement sequence/lag graph with cycle diagnostics
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `SCHED-CAL` - implement calendars and recurrence expansion with budgets
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `SCHED-EVENT` - implement events and event times
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `SCHED-QUERY` - build deterministic timeline queries independent of cost/resources
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.

SCHED-ROOT - cargo test -p ifc-schedule (17 passing) - IfcWorkPlan and
IfcWorkSchedule share the IfcWorkControl slot layout; both are IfcControl, so
membership is IfcRelAssignsToControl, not IfcRelAggregates.

SCHED-TASK - cargo test -p ifc-schedule (17 passing) - IfcTask.IsMilestone is
slot 9, after IfcProcess' Identification/LongDescription and IfcTask's own
Status/WorkMethod. WR1 (a milestone has no duration) is enforced in crate logic
because IfcOpenShell validation does not check it.

SCHED-SEQ - cargo test -p ifc-schedule (17 passing) - IfcRelSequence stores
RelatingProcess 4 / RelatedProcess 5, and unlike IfcRelConnectsPorts this order
IS directional: relating is the predecessor. Lag is a signed IfcLagTime, so a
negative lag legitimately means overlap.

SCHED-CAL - cargo test -p ifc-schedule (17 passing) - IfcRecurrencePattern has
Position at slot 4, so Interval is 5 and Occurrences 6. A pattern with neither
Occurrences nor a bounding period is unbounded and is reported, never expanded.

SCHED-EVENT - cargo test -p ifc-schedule (17 passing) - IfcEventTypeEnum has no
MILESTONE member; the valid tokens are STARTEVENT/ENDEVENT/INTERMEDIATEEVENT.

SCHED-QUERY - cargo test -p ifc-schedule (17 passing) - execution_order runs
Kahn's algorithm with the ready set kept in file order, so a schedule with
independent chains has one stated answer rather than a valid-but-arbitrary one.
A cycle returns the offending path instead of stalling.
