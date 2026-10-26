//! Catalog member rule ids (#537): a property the set's template does not
//! declare, and a value whose type differs from the template's.
//!
//! The catalog is a stand-in declaring one set, `Pset_Test`, with one
//! `IfcBoolean` property, `IsExternal`. Each failing fixture differs from
//! its passing one only in the property's name or in the type it holds.

use ifc_model::{EntityId, Model, Value};
use ifc_schema::SchemaVersion;
use ifc_validate::{
    validate_with_options, CatalogMember, CatalogMemberForm, CatalogSet, PropertySetCatalog,
    Report, ValidateOptions,
};

use super::cases::Case;
use super::fixtures::{entity, text, typed, GUID_A};

struct Catalog;

impl PropertySetCatalog for Catalog {
    fn defines(&self, _: SchemaVersion, name: &str) -> Option<bool> {
        Some(name == "Pset_Test")
    }

    fn template(&self, _: SchemaVersion, name: &str) -> Option<CatalogSet> {
        (name == "Pset_Test").then(|| {
            CatalogSet::property(vec![CatalogMember::new(
                "IsExternal",
                CatalogMemberForm::SingleValue {
                    data_type: Some("IfcBoolean".into()),
                },
            )])
        })
    }
}

/// A `Pset_Test` holding one single-value property `name` of `value`.
fn set_with(name: &str, value: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(
            schema,
            "IFCPROPERTYSINGLEVALUE",
            &[("Name", text(name)), ("NominalValue", value)],
        ),
    );
    model.insert(
        EntityId(2),
        entity(
            schema,
            "IFCPROPERTYSET",
            &[
                ("GlobalId", text(GUID_A)),
                ("Name", text("Pset_Test")),
                ("HasProperties", Value::List(vec![Value::Ref(EntityId(1))])),
            ],
        ),
    );
    validate_with_options(&model, schema, &ValidateOptions::new().catalog(&Catalog))
}

fn boolean() -> Value {
    typed("IFCBOOLEAN", Value::Bool(true))
}

pub const CASES: &[Case] = &[
    Case {
        rule: "property.undeclared_member",
        form: "a property the catalog's set template does not declare",
        fails: || set_with("MyOwnProp", boolean()),
        passes: || set_with("IsExternal", boolean()),
    },
    Case {
        rule: "property.member_type",
        form: "an IfcLabel where the template declares an IfcBoolean",
        fails: || set_with("IsExternal", typed("IFCLABEL", text("yes"))),
        passes: || set_with("IsExternal", boolean()),
    },
];
