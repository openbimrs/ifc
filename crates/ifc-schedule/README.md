# ifc-schedule

Construction scheduling as a borrowed view over the IFC model: work plans and
schedules, tasks and task times, sequences with lag, calendars, recurrence and
events, plus the sequence graph and a deterministic execution order. Dates and
durations are returned as authored ISO 8601 strings, not computed.

```bash
cargo add ifc-schedule
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`schedule` feature.

- API documentation: [docs.rs/ifc-schedule](https://docs.rs/ifc-schedule)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-schedule)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- The crate depends on `ifc-model` alone. `ifc-schema` may be added only
  for generic schema validation, never to drive the projections, which
  read fixed, documented slots.
