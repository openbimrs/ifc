//! Small IFC4X3 alignment models, built record by record.
//!
//! The hierarchy and stationing tests need several alignments wired
//! through `IfcRelNests`, `IfcRelAggregates` and `IfcRelPositions`. The
//! records are written with the crate's own authoring where it has a
//! writer, and as plain entities for the relationships it leaves to
//! `ifc-author`. Slot positions follow `IFC4X3_ADD2.exp`.

// Each test crate uses a different subset.
#![allow(dead_code)]

use std::sync::Arc;

use ifc_alignment::{
    alignment, alignment_segment, axis2_placement_linear, cant_layout, horizontal_layout,
    linear_placement, point_by_distance, referent, stationing, vertical_layout, AlignmentUnits,
};
use ifc_model::{Entity, EntityId, Header, Model, Transaction, Value};

pub fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

/// A transaction over an empty IFC4X3_ADD2 model, with a GUID counter.
pub struct Builder {
    pub model: Model,
    pub tx: Transaction,
    next_guid: u32,
}

impl Builder {
    pub fn new() -> Self {
        Self::with_schema("IFC4X3_ADD2")
    }

    pub fn with_schema(token: &str) -> Self {
        let mut model = Model::new();
        *model.header_mut() = Header {
            schema: vec![token.to_owned()],
            ..Header::default()
        };
        let tx = Transaction::new(&model);
        Self {
            model,
            tx,
            next_guid: 0,
        }
    }

    /// A fresh 22-character GlobalId.
    pub fn guid(&mut self) -> String {
        self.next_guid += 1;
        format!("{:0>22}", self.next_guid)
    }

    pub fn finish(self) -> Model {
        let Self { mut model, tx, .. } = self;
        tx.commit(&mut model).expect("commit");
        model
    }

    pub fn alignment(&mut self, name: &str) -> EntityId {
        let guid = self.guid();
        alignment(&mut self.tx, &guid, Some(name), None).expect("alignment")
    }

    pub fn horizontal(&mut self) -> EntityId {
        let guid = self.guid();
        horizontal_layout(&mut self.tx, &guid, None).expect("horizontal")
    }

    pub fn vertical(&mut self) -> EntityId {
        let guid = self.guid();
        vertical_layout(&mut self.tx, &guid, None).expect("vertical")
    }

    pub fn cant(&mut self) -> EntityId {
        let guid = self.guid();
        cant_layout(&mut self.tx, &guid, None, 1.5).expect("cant")
    }

    /// `IfcRelNests`: `children` nested under `parent`, in this order.
    pub fn nest(&mut self, parent: EntityId, children: &[EntityId]) -> EntityId {
        self.relation("IFCRELNESTS", parent, children)
    }

    /// `IfcRelAggregates`: `children` aggregated under `parent`.
    pub fn aggregate(&mut self, parent: EntityId, children: &[EntityId]) -> EntityId {
        self.relation("IFCRELAGGREGATES", parent, children)
    }

    /// `IfcRelPositions`: `products` positioned by `element`.
    pub fn positions(&mut self, element: EntityId, products: &[EntityId]) -> EntityId {
        self.relation("IFCRELPOSITIONS", element, products)
    }

    fn relation(&mut self, type_name: &str, relating: EntityId, related: &[EntityId]) -> EntityId {
        let guid = self.guid();
        self.tx.create(Entity::new(
            type_name,
            vec![
                Value::Text(guid.into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(relating),
                Value::List(related.iter().copied().map(Value::Ref).collect()),
            ],
        ))
    }

    /// An `IfcProject`, for aggregating root alignments under.
    pub fn project(&mut self) -> EntityId {
        let guid = self.guid();
        let mut attrs = vec![Value::Null; 9];
        attrs[0] = Value::Text(guid.into());
        self.tx.create(Entity::new("IFCPROJECT", attrs))
    }

    /// A basis curve for referent placements. Its geometry is never read.
    pub fn curve(&mut self) -> EntityId {
        self.tx
            .create(Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]))
    }

    /// An `IfcReferent` at `distance` along `curve`, without stationing.
    pub fn marker(&mut self, curve: EntityId, distance: f64) -> EntityId {
        let point =
            point_by_distance(&mut self.tx, distance, (None, None, None), curve).expect("point");
        let axis = axis2_placement_linear(&mut self.tx, point, None, None).expect("axis");
        let placement = linear_placement(&mut self.tx, axis, None, None).expect("placement");
        let guid = self.guid();
        referent(&mut self.tx, &guid, None, Some("STATION"), Some(placement)).expect("referent")
    }

    /// A stationing `IfcReferent` at `distance` along `curve`.
    pub fn station(
        &mut self,
        curve: EntityId,
        distance: f64,
        station: f64,
        incoming: Option<f64>,
        increasing: Option<bool>,
    ) -> EntityId {
        let marker = self.marker(curve, distance);
        let (pset, rel) = (self.guid(), self.guid());
        stationing(
            &mut self.tx,
            &pset,
            &rel,
            marker,
            station,
            incoming,
            increasing,
        )
        .expect("stationing");
        marker
    }

    /// An `IfcAlignmentSegment` wrapping a vertical parameter segment.
    pub fn vertical_segment(
        &mut self,
        start: f64,
        length: f64,
        height: f64,
        grades: (f64, f64),
        radius: Option<f64>,
        kind: &str,
    ) -> EntityId {
        let parameters = self.tx.create(Entity::new(
            "IFCALIGNMENTVERTICALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Real(start),
                Value::Real(length),
                Value::Real(height),
                Value::Real(grades.0),
                Value::Real(grades.1),
                radius.map_or(Value::Null, Value::Real),
                Value::Enum(Arc::from(kind)),
            ],
        ));
        self.wrap(parameters)
    }

    /// An `IfcAlignmentSegment` wrapping a constant cant parameter segment.
    pub fn cant_segment(&mut self, start: f64, length: f64) -> EntityId {
        let parameters = self.tx.create(Entity::new(
            "IFCALIGNMENTCANTSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Real(start),
                Value::Real(length),
                Value::Real(0.0),
                Value::Null,
                Value::Real(0.0),
                Value::Null,
                Value::Enum(Arc::from("CONSTANTCANT")),
            ],
        ));
        self.wrap(parameters)
    }

    /// An `IfcAlignmentSegment` wrapping a straight horizontal segment.
    pub fn horizontal_segment(&mut self, length: f64) -> EntityId {
        let point = self.tx.create(Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ));
        let parameters = self.tx.create(Entity::new(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(point),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(length),
                Value::Null,
                Value::Enum(Arc::from("LINE")),
            ],
        ));
        self.wrap(parameters)
    }

    fn wrap(&mut self, parameters: EntityId) -> EntityId {
        let guid = self.guid();
        alignment_segment(&mut self.tx, &guid, parameters).expect("segment")
    }

    /// The `DesignParameters` an `IfcAlignmentSegment` wrapper points at.
    pub fn parameters_of(model: &Model, segment: EntityId) -> EntityId {
        model
            .get(segment)
            .and_then(|entity| entity.attributes[7].as_ref_id())
            .expect("wrapper")
    }
}
