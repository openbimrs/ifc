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
  work-control, sequence and assignment readers find every attribute by
  name in that release's table (#212). The calendar and event readers,
  whose entities IFC2X3 does not declare, still read the IFC4 slots.
