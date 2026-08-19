# IFC implementation plan (superseded)

Status: superseded on 2026-08-19 by the progressive package plans.

The old plan mixed completed infrastructure milestones with speculative work for
every domain crate. That made it stale ambient context and gave implementers no
local place to record task evidence.

Use:

- `packages/ifc/AGENTS.md` for stable package architecture;
- `packages/ifc/PLAN.md` for cross-crate integration order;
- `packages/ifc/<crate>/AGENTS.md` for the crate contract;
- `packages/ifc/<crate>/PLAN.md` for checkable implementation work;
- a deeper paired `AGENTS.md` / `PLAN.md` under complex modules when present.

Do not add implementation progress here. Update the nearest owning `PLAN.md`.
