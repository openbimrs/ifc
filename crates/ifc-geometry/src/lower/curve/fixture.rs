//! Small IFC4X3 curve models for unit tests, built entity by entity.

use ifc_model::{Entity, EntityId, Model, Value};

use crate::solid::testkit::{entity, n, r};
use crate::units::UnitScale;

pub(super) const METRES: UnitScale = UnitScale {
    length_to_metres: 1.0,
    angle_to_radians: 1.0,
};

/// Builder for small segment models; ids are handed out in order.
pub(super) struct Builder {
    pub(super) model: Model,
    next: u64,
}

impl Builder {
    pub(super) fn new() -> Self {
        Self {
            model: Model::new(),
            next: 1,
        }
    }

    pub(super) fn add(&mut self, entity: Entity) -> u64 {
        let id = self.next;
        self.model.insert(EntityId(id), entity);
        self.next += 1;
        id
    }

    pub(super) fn point(&mut self, coordinates: &[f64]) -> u64 {
        self.add(entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(coordinates.iter().copied().map(n).collect())],
        ))
    }

    pub(super) fn placement(&mut self, origin: [f64; 2], direction: [f64; 2]) -> u64 {
        let location = self.point(&origin);
        let dir = self.add(entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(direction[0]), n(direction[1])])],
        ));
        self.add(entity("IFCAXIS2PLACEMENT2D", vec![r(location), r(dir)]))
    }

    pub(super) fn line(&mut self) -> u64 {
        let origin = self.point(&[0.0, 0.0]);
        let dir = self.add(entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0)])],
        ));
        let vector = self.add(entity("IFCVECTOR", vec![r(dir), n(1.0)]));
        self.add(entity("IFCLINE", vec![r(origin), r(vector)]))
    }

    pub(super) fn circle(&mut self, radius: f64) -> u64 {
        let position = self.placement([3.0, -7.0], [0.0, 1.0]);
        self.add(entity("IFCCIRCLE", vec![r(position), n(radius)]))
    }

    pub(super) fn clothoid(&mut self, constant: f64) -> u64 {
        let position = self.placement([0.0, 0.0], [1.0, 0.0]);
        self.add(entity("IFCCLOTHOID", vec![r(position), n(constant)]))
    }

    /// `x = u`, `y = coefficients(u)`: a parabola in its own frame.
    pub(super) fn parabola(&mut self, coefficients: &[f64]) -> u64 {
        let position = self.placement([0.0, 0.0], [1.0, 0.0]);
        self.add(entity(
            "IFCPOLYNOMIALCURVE",
            vec![
                r(position),
                Value::List(vec![n(0.0), n(1.0)]),
                Value::List(coefficients.iter().copied().map(n).collect()),
                Value::Null,
            ],
        ))
    }

    /// An `IfcCurveSegment` starting at its parent's origin.
    pub(super) fn segment_at(
        &mut self,
        origin: [f64; 2],
        direction: [f64; 2],
        run: f64,
        parent: u64,
    ) -> u64 {
        let place = self.placement(origin, direction);
        self.segment(place, length(0.0), length(run), parent)
    }

    /// An open `IfcCompositeCurve` over `segments`.
    pub(super) fn composite(&mut self, segments: &[u64]) -> u64 {
        self.add(entity(
            "IFCCOMPOSITECURVE",
            vec![
                Value::List(segments.iter().copied().map(r).collect()),
                Value::Bool(false),
            ],
        ))
    }

    pub(super) fn segment(
        &mut self,
        placement: u64,
        start: Value,
        length: Value,
        parent: u64,
    ) -> u64 {
        self.add(entity(
            "IFCCURVESEGMENT",
            vec![
                Value::Enum("CONTINUOUS".into()),
                r(placement),
                start,
                length,
                r(parent),
            ],
        ))
    }
}

pub(super) fn length(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(n(value)),
    }
}

pub(super) fn parameter(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(n(value)),
    }
}
