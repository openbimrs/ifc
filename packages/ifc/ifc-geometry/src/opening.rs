//! `IfcRelVoidsElement` void cutting.
//!
//! Windows and doors cut openings out of walls. This is the single most
//! important boolean operation in IFC and the reason a robust mesh boolean is
//! the project's decisive dependency (see `docs/adr/0003`).
//!
//! # Pitfall
//!
//! Two openings that overlap must produce the same result regardless of cut
//! order (see the `issue_2019_wall_two_overlapping_openings` fixture).
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
