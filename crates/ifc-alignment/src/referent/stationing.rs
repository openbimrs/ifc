//! One alignment's stationing, and station to distance-along lookup.
//!
//! An alignment states its stationing through the `IfcReferent`s it nests
//! (`IfcRelNests`, "Object Nesting") or positions (`IfcRelPositions`), each
//! carrying `Pset_Stationing`. Referents of other alignments never enter
//! the table, which is what keeps two alignments' chainages apart.
//!
//! # The mapping
//!
//! Sorted by distance along, referent `i` at distance `d_i` with
//! `Station = s_i` governs `[d_i, d_{i+1})` (the last one is open-ended):
//!
//! ```text
//! station(d) = s_i + k_i * (d - d_i),   k_i = +1 if HasIncreasingStation else -1
//! ```
//!
//! At an equation the referent's `Station` is the value ahead of it and
//! its `IncomingStation` the value the previous range reaches there. Both
//! are checked: `IncomingStation` must equal `s_{i-1} + k_{i-1} * (d_i -
//! d_{i-1})`, and a referent without one must continue that value, within
//! [`STATION_TOLERANCE`]. A contradiction is refused rather than resolved
//! in favour of either number.
//!
//! The inverse treats each range as closed at its end, so the station a
//! range reaches at an equation (the incoming value) maps back to that
//! equation's distance. A station that occurs in several ranges, as after
//! an equation that steps back, is reported by [`Stationing::distances_at`]
//! and refused by [`Stationing::distance_at`].

use ifc_model::{EntityId, Model};

use super::station::{resolve_referent_stationing, StationEquation};
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;
use crate::view::AlignmentView;

/// How far, in metres, a referent's stated station may differ from the
/// value carried from the previous referent before the two are treated as
/// contradictory.
///
/// One millimetre: stations and distances are authored independently and
/// commonly rounded to the millimetre, so a tighter bound would refuse
/// sound files, while any real equation is a deliberate, larger jump that
/// is stated through `IncomingStation`.
pub const STATION_TOLERANCE: f64 = 1e-3;

/// The stationing of one `IfcAlignment`: its station equations in distance
/// order, and lookups across them.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Stationing {
    /// The `IfcAlignment` this stationing belongs to.
    pub alignment: EntityId,
    equations: Vec<StationEquation>,
}

impl Stationing {
    /// Resolve the stationing of `alignment` from the referents it nests
    /// or positions.
    ///
    /// Referents without `Pset_Stationing` (a bridge pier marker, say) are
    /// not part of the stationing and are skipped. An alignment with no
    /// stationing referent resolves to an empty table, whose lookups are
    /// refused.
    ///
    /// # Errors
    ///
    /// Refuses a model that is not IFC4X3, an `alignment` that is not an
    /// `IfcAlignment`, a malformed relationship or referent, two stationing
    /// referents at one distance, a non-finite station, and a station
    /// that contradicts the one carried from the previous referent.
    pub fn resolve(
        model: &Model,
        alignment: EntityId,
        units: AlignmentUnits,
    ) -> AlignmentResult<Self> {
        let view = AlignmentView::for_model(model)?;
        let hierarchy = view.hierarchy(alignment)?;
        let mut referents = hierarchy.referents;
        for id in view.filter_is_a(hierarchy.positioned, "IfcReferent") {
            if !referents.contains(&id) {
                referents.push(id);
            }
        }
        let mut equations = Vec::with_capacity(referents.len());
        for referent in referents {
            if let Some(equation) = resolve_referent_stationing(model, referent, units)? {
                equations.push(equation);
            }
        }
        Self::from_equations(alignment, equations)
    }

    fn from_equations(
        alignment: EntityId,
        mut equations: Vec<StationEquation>,
    ) -> AlignmentResult<Self> {
        for equation in &equations {
            let finite = equation.distance_along.is_finite()
                && equation.station.is_finite()
                && equation.incoming_station.is_none_or(f64::is_finite);
            if !finite {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(equation.referent),
                    rule: "a stationing referent's distance and stations must be finite",
                });
            }
        }
        equations.sort_by(|a, b| a.distance_along.total_cmp(&b.distance_along));
        for pair in equations.windows(2) {
            let [previous, next] = pair else {
                unreachable!("windows(2) yields pairs")
            };
            if next.distance_along - previous.distance_along <= STATION_TOLERANCE {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(next.referent),
                    rule:
                        "two stationing referents of one alignment sit at the same distance along",
                });
            }
            let carried = station_in(previous, next.distance_along);
            let stated = next.incoming_station.unwrap_or(next.station);
            if (stated - carried).abs() > STATION_TOLERANCE {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(next.referent),
                    rule: if next.incoming_station.is_some() {
                        "IncomingStation disagrees with the station carried from the previous referent"
                    } else {
                        "a referent without IncomingStation must continue the previous station"
                    },
                });
            }
        }
        Ok(Self {
            alignment,
            equations,
        })
    }

    /// The stationing referents, ascending in distance along.
    #[must_use]
    pub fn equations(&self) -> &[StationEquation] {
        &self.equations
    }

    /// The station at `distance_along` (metres along the alignment).
    ///
    /// At an equation the station ahead of it applies.
    ///
    /// # Errors
    ///
    /// [`AlignmentError::OutOfRange`] for a non-finite distance, an empty
    /// table, or a distance before the first stationing referent.
    pub fn station_at(&self, distance_along: f64) -> AlignmentResult<f64> {
        let out_of_range = AlignmentError::OutOfRange {
            entity: self.alignment,
            quantity: "distance along",
            value: distance_along,
        };
        if !distance_along.is_finite() {
            return Err(out_of_range);
        }
        let governing = self
            .equations
            .iter()
            .rev()
            .find(|equation| equation.distance_along <= distance_along)
            .ok_or(out_of_range)?;
        Ok(station_in(governing, distance_along))
    }

    /// Every distance along labelled `station`, ascending.
    ///
    /// Usually one. Several when an equation steps the station back, so
    /// that the same value occurs in two ranges.
    ///
    /// # Errors
    ///
    /// [`AlignmentError::OutOfRange`] for a non-finite station and a
    /// station no range reaches.
    pub fn distances_at(&self, station: f64) -> AlignmentResult<Vec<f64>> {
        let out_of_range = AlignmentError::OutOfRange {
            entity: self.alignment,
            quantity: "station",
            value: station,
        };
        if !station.is_finite() {
            return Err(out_of_range);
        }
        let mut distances: Vec<f64> = Vec::new();
        for (index, equation) in self.equations.iter().enumerate() {
            let span = self.equations.get(index + 1).map_or(f64::INFINITY, |next| {
                next.distance_along - equation.distance_along
            });
            let offset = (station - equation.station) * direction(equation);
            let tolerance = 1e-9 * station.abs().max(1.0);
            if offset < -tolerance || offset > span + tolerance {
                continue;
            }
            let distance = equation.distance_along + offset.clamp(0.0, span);
            let duplicate = distances
                .last()
                .is_some_and(|last| (distance - last).abs() <= tolerance);
            if !duplicate {
                distances.push(distance);
            }
        }
        if distances.is_empty() {
            return Err(out_of_range);
        }
        Ok(distances)
    }

    /// The one distance along labelled `station`.
    ///
    /// # Errors
    ///
    /// As [`Self::distances_at`], and [`AlignmentError::AmbiguousStation`]
    /// when the station labels several distances.
    pub fn distance_at(&self, station: f64) -> AlignmentResult<f64> {
        let distances = self.distances_at(station)?;
        match distances.as_slice() {
            [only] => Ok(*only),
            _ => Err(AlignmentError::AmbiguousStation {
                alignment: self.alignment,
                station,
                distances,
            }),
        }
    }
}

/// `+1` for increasing stationing, `-1` for decreasing.
fn direction(equation: &StationEquation) -> f64 {
    if equation.has_increasing_station {
        1.0
    } else {
        -1.0
    }
}

/// The station `equation`'s range assigns to `distance_along`.
fn station_in(equation: &StationEquation, distance_along: f64) -> f64 {
    equation.station + direction(equation) * (distance_along - equation.distance_along)
}
