//! Classification references (feature `classification`, #123).
//!
//! The facade's classification view resolves the classifications that
//! apply to an object: those associated with it directly, and those
//! associated with its type object. Each crosses as one record naming the
//! relationship, the classification it points at (a reference, a whole
//! system, or an IFC2X3 notation), the reference's code and name, the
//! references above it, and the system at the top. Records are read
//! against the release the header declares, IFC2X3, IFC4 or IFC4X3; any
//! other is refused with `unsupported-schema`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// One classification that applies to an object.
#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    /// The `IfcRelAssociatesClassification`.
    pub relationship: u64,
    /// The relationship's `GlobalId`.
    pub global_id: Option<String>,
    /// `occurrence`: associated with the queried object; `type`: with its
    /// type object.
    pub source: String,
    /// For `type`, the type object's id.
    pub type_object: Option<u64>,
    /// The `RelatingClassification`.
    pub target: u64,
    /// `reference` (an `IfcClassificationReference`), `system` (an
    /// `IfcClassification`) or `notation` (an IFC2X3
    /// `IfcClassificationNotation`).
    pub kind: String,
    /// A reference's `Identification` (IFC2X3 `ItemReference`).
    pub identification: Option<String>,
    /// A reference's `Name`.
    pub name: Option<String>,
    /// A reference's `Location`.
    pub location: Option<String>,
    /// A notation's facet values, in file order; empty otherwise.
    pub notation: Vec<String>,
    /// The references above a `reference` target, nearest first.
    pub parents: Vec<u64>,
    /// The system at the top: the target itself for `system`, the end of
    /// the `ReferencedSource` chain for `reference` when it ends in one.
    pub system: Option<ClassificationSystem>,
}

/// An `IfcClassification`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationSystem {
    /// Its entity id.
    pub id: u64,
    /// `Name`.
    pub name: String,
    /// `Source`: the publisher.
    pub source: Option<String>,
    /// `Edition`.
    pub edition: Option<String>,
}

impl IfcModel {
    /// The classifications that apply to `object`: its own associations,
    /// then its type object's, in relationship id order.
    ///
    /// Refused with `missing-entity` for an id not in the model,
    /// `unsupported-schema` for a release other than IFC2X3, IFC4 or
    /// IFC4X3, `invalid-model` for malformed or ambiguous associations,
    /// `missing-reference` for a dangling one, `budget-exceeded` for a
    /// reference chain that cycles or runs deeper than 64, and
    /// `feature-disabled` without the `classification` feature.
    pub fn classifications(&self, object: u64) -> Result<Vec<Classification>, BindingError> {
        #[cfg(feature = "classification")]
        {
            read::classifications(self, object)
        }
        #[cfg(not(feature = "classification"))]
        {
            let _ = object;
            Err(BindingError::FeatureDisabled("classification"))
        }
    }
}

#[cfg(feature = "classification")]
mod read {
    use ifc::classification::{
        classification_schema, ClassificationAssignment, ClassificationError, ClassificationView,
    };
    use ifc::{Budget, EntityId};

    use super::{Classification, ClassificationSystem};
    use crate::{BindingError, IfcModel};

    pub(super) fn classifications(
        model: &IfcModel,
        object: u64,
    ) -> Result<Vec<Classification>, BindingError> {
        if !model.inner.contains(EntityId(object)) {
            return Err(BindingError::MissingEntity(object));
        }
        classification_schema(&model.inner).map_err(error)?;
        let view = ClassificationView::new(&model.inner);
        let effective = view
            .effective_classifications(EntityId(object))
            .map_err(error)?;
        let mut out = Vec::new();
        for assignment in &effective.occurrence {
            out.push(one(model, view, *assignment, "occurrence", None)?);
        }
        let type_object = effective.type_object.map(|EntityId(id)| id);
        for assignment in &effective.inherited {
            out.push(one(model, view, *assignment, "type", type_object)?);
        }
        Ok(out)
    }

    fn one(
        model: &IfcModel,
        view: ClassificationView<'_>,
        assignment: ClassificationAssignment<'_>,
        source: &str,
        type_object: Option<u64>,
    ) -> Result<Classification, BindingError> {
        let target = assignment.relating_classification_id().map_err(error)?;
        let mut record = Classification {
            relationship: assignment.id().0,
            global_id: Some(assignment.global_id().map_err(error)?.to_owned()),
            source: source.to_owned(),
            type_object,
            target: target.0,
            kind: String::new(),
            identification: None,
            name: None,
            location: None,
            notation: Vec::new(),
            parents: Vec::new(),
            system: None,
        };
        let type_name = model.type_of(target.0)?;
        match type_name {
            "IFCCLASSIFICATIONREFERENCE" => {
                record.kind = "reference".to_owned();
                let hierarchy = view
                    .hierarchy_from(target, Budget::DEFAULT)
                    .map_err(error)?;
                if let Some(reference) = hierarchy.references.first() {
                    let owned = |text: Option<&str>| text.map(str::to_owned);
                    record.identification = owned(reference.identification().map_err(error)?);
                    record.name = owned(reference.name().map_err(error)?);
                    record.location = owned(reference.location().map_err(error)?);
                }
                record.parents = hierarchy
                    .references
                    .iter()
                    .skip(1)
                    .map(|reference| reference.id().0)
                    .collect();
                record.system = hierarchy.system.map(system).transpose()?;
            }
            "IFCCLASSIFICATION" => {
                record.kind = "system".to_owned();
                let found = view.systems().find(|candidate| candidate.id() == target);
                record.system = found.map(system).transpose()?;
            }
            "IFCCLASSIFICATIONNOTATION" => {
                record.kind = "notation".to_owned();
                record.notation = view
                    .notation_values(target)
                    .map_err(error)?
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
            }
            other => {
                return Err(BindingError::Unsupported(format!(
                    "#{} classifies by {other}, which this binding does not read",
                    assignment.id().0
                )))
            }
        }
        Ok(record)
    }

    fn system(
        system: ifc::classification::ClassificationSystem<'_>,
    ) -> Result<ClassificationSystem, BindingError> {
        let owned = |text: Option<&str>| text.map(str::to_owned);
        Ok(ClassificationSystem {
            id: system.id().0,
            name: system.name().map_err(error)?.to_owned(),
            source: owned(system.source().map_err(error)?),
            edition: owned(system.edition().map_err(error)?),
        })
    }

    fn error(error: ClassificationError) -> BindingError {
        use ClassificationError as E;
        let detail = error.to_string();
        match error {
            E::UnsupportedSchema { schema } => BindingError::UnsupportedSchema(schema),
            E::MultipleSchemas { .. } => BindingError::UnsupportedSchema(detail),
            E::UnknownEntity { id } => BindingError::MissingEntity(id.0),
            E::DanglingReference { .. } => BindingError::MissingReference(detail),
            E::Cycle { .. } | E::BudgetExceeded { .. } => BindingError::BudgetExceeded(detail),
            E::WrongEntityType { .. } => BindingError::WrongEntityType(detail),
            _ => BindingError::InvalidModel(detail),
        }
    }
}

impl ToRecord for Classification {
    fn to_record(&self) -> Record {
        Record::new(
            "Classification",
            vec![
                ("relationship", Field::Id(self.relationship)),
                ("global_id", Field::text(self.global_id.clone())),
                ("source", Field::Text(self.source.clone())),
                ("type_object", Field::id(self.type_object)),
                ("target", Field::Id(self.target)),
                ("kind", Field::Text(self.kind.clone())),
                ("identification", Field::text(self.identification.clone())),
                ("name", Field::text(self.name.clone())),
                ("location", Field::text(self.location.clone())),
                (
                    "notation",
                    Field::List(self.notation.iter().cloned().map(Field::Text).collect()),
                ),
                ("parents", Field::ids(self.parents.iter().copied())),
                ("system", Field::record(self.system.as_ref())),
            ],
        )
    }
}

impl ToRecord for ClassificationSystem {
    fn to_record(&self) -> Record {
        Record::new(
            "ClassificationSystem",
            vec![
                ("id", Field::Id(self.id)),
                ("name", Field::Text(self.name.clone())),
                ("source", Field::text(self.source.clone())),
                ("edition", Field::text(self.edition.clone())),
            ],
        )
    }
}
