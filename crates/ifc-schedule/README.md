# ifc-schedule

Construction scheduling as a borrowed view over the IFC model: work plans and
schedules, tasks and task times, sequences with lag, calendars, recurrence and
events, plus the sequence graph and a deterministic execution order. Dates and
durations are returned as authored (ISO 8601 strings, or IFC2X3 date
records and time measures), not computed.

```bash
cargo add ifc-schedule
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`schedule` feature.

- API documentation: [docs.rs/ifc-schedule](https://docs.rs/ifc-schedule)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-schedule)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- The crate depends on `ifc-model` and, for the bundled release tables,
  `ifc-schema`. The tables lay out the records the `*_with_owner_history`
  writers author in the model's declared release (#202), and the task,
  work-control, sequence, assignment, calendar and event readers find
  every attribute by name in that release's table (#212, #234), so IFC4X3's
  `IfcWorkTime.StartDate`/`FinishDate` read as IFC4's `Start`/`Finish`.
  A header they cannot bind (IFC4X1, IFC4X2, several schemas) is refused
  with `ScheduleReadError`.
- A task's times are its `IfcTaskTime` or `IfcTaskTimeRecurring` in IFC4
  and IFC4X3 (`Task::time`, `TaskTime::recurrence`), and in IFC2X3 the
  `IfcScheduleTimeControl` its `IfcRelAssignsTasks` names
  (`Task::schedule_time_controls`) (#235).
