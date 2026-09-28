//! `assign_cost_quantities` accepts what the declared release declares
//! (#203).
//!
//! `IfcCostItem.CostQuantities` is `OPTIONAL SET [1:?] OF
//! IfcPhysicalQuantity` in IFC4 ADD2 TC1 and IFC4X3 ADD2, and IFC4X3 adds
//! `IfcQuantityNumber` under `IfcPhysicalSimpleQuantity`. IFC2X3 TC1
//! `IfcCostItem` declares no `CostQuantities` at all. The accepted set is
//! the release's own subtype tree, never a fixed list.

use ifc_cost::mutation::{
    assign_cost_quantities, create_cost_item, create_cost_item_with_owner_history, create_quantity,
    CostAuthoringError, CostItemDraft, QuantityDraft, QuantityKind,
};
use ifc_cost::quantity::CostQuantity;
use ifc_cost::{CostItem, SchemaVersion};
use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::for_version;
use ifc_step::StepCodec;

const GUID: &str = "0YvctVUKr0kugbFTf53O08";

fn declaring(schema: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema.to_owned()];
    model
}

fn item() -> CostItemDraft<'static> {
    CostItemDraft {
        global_id: GUID,
        name: Some("Item"),
        ..CostItemDraft::default()
    }
}

fn quantity(kind: QuantityKind, value: f64) -> QuantityDraft<'static> {
    QuantityDraft {
        kind,
        name: "Measured",
        description: None,
        unit: None,
        value,
        formula: None,
    }
}

/// The #203 case: a Number quantity attaches in IFC4X3 and reads back.
#[test]
fn a_number_quantity_attaches_to_a_cost_item_in_ifc4x3() {
    let mut model = declaring("IFC4X3_ADD2");
    let mut tx = Transaction::new(&model);
    let cost_item = create_cost_item(&mut tx, &model, item()).expect("item");
    let number = create_quantity(&mut tx, &model, quantity(QuantityKind::Number, -2.5))
        .expect("IfcQuantityNumber exists in IFC4X3");
    let area = create_quantity(&mut tx, &model, quantity(QuantityKind::Area, 12.0)).unwrap();
    assign_cost_quantities(&mut tx, &model, cost_item, &[number, area])
        .expect("IfcQuantityNumber is an IfcPhysicalQuantity in IFC4X3");
    tx.commit(&mut model).expect("commit");

    let bytes = StepCodec.write_bytes(&model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    let view = CostItem::new(cost_item, back.get(cost_item).unwrap());
    assert_eq!(view.quantity_refs(), vec![number, area]);
    let number_view = CostQuantity::new(number, back.get(number).unwrap());
    assert_eq!(number_view.value(), Some(-2.5));
}

/// Every instantiable `IfcPhysicalQuantity` subtype a release declares is
/// accepted there, read from that release's table.
#[test]
fn every_declared_physical_quantity_is_accepted() {
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let table = for_version(version).unwrap();
        let subtypes: Vec<String> = table
            .subtypes("IFCPHYSICALQUANTITY")
            .into_iter()
            .filter(|name| table.entity(name).is_some_and(|e| !e.abstract_))
            .map(str::to_ascii_uppercase)
            .collect();
        assert_eq!(
            subtypes.iter().any(|s| s == "IFCQUANTITYNUMBER"),
            version == SchemaVersion::Ifc4x3,
            "{schema}: only IFC4X3 declares IfcQuantityNumber"
        );
        let mut model = declaring(schema);
        let ids: Vec<EntityId> = (100..)
            .zip(&subtypes)
            .map(|(n, name)| {
                let id = EntityId(n);
                model.insert(id, Entity::new(name.as_str(), vec![Value::Null]));
                id
            })
            .collect();
        let mut tx = Transaction::new(&model);
        let cost_item = create_cost_item(&mut tx, &model, item()).unwrap();
        assign_cost_quantities(&mut tx, &model, cost_item, &ids)
            .unwrap_or_else(|e| panic!("{schema}: {e}"));
    }
}

/// IFC4 keeps refusing `IfcQuantityNumber`, which it does not declare, and
/// every release refuses what is no physical quantity. Nothing is staged.
#[test]
fn what_the_release_does_not_declare_is_refused() {
    for (schema, number_refused) in [("IFC4", true), ("IFC4X3_ADD2", false)] {
        let mut model = declaring(schema);
        model.insert(
            EntityId(100),
            Entity::new("IFCQUANTITYNUMBER", vec![Value::Null]),
        );
        model.insert(EntityId(101), Entity::new("IFCWALL", vec![Value::Null]));
        let mut tx = Transaction::new(&model);
        let cost_item = create_cost_item(&mut tx, &model, item()).unwrap();
        for (target, refused) in [(EntityId(100), number_refused), (EntityId(101), true)] {
            let staged = tx.len();
            let result = assign_cost_quantities(&mut tx, &model, cost_item, &[target]);
            if refused {
                assert!(
                    matches!(
                        result,
                        Err(CostAuthoringError::WrongReferenceType {
                            expected: "IFCPHYSICALQUANTITY",
                            ..
                        })
                    ),
                    "{schema} {target}: {result:?}"
                );
                assert_eq!(tx.len(), staged, "a refusal staged nothing");
            } else {
                result.expect("accepted");
            }
        }
    }
}

/// IFC2X3 `IfcCostItem` has no `CostQuantities` slot: refused, not written
/// past the five-attribute record.
#[test]
fn ifc2x3_cost_items_carry_no_quantities() {
    let mut model = declaring("IFC2X3");
    model.insert(
        EntityId(5),
        Entity::new("IFCOWNERHISTORY", vec![Value::Null; 8]),
    );
    let mut tx = Transaction::new(&model);
    let cost_item =
        create_cost_item_with_owner_history(&mut tx, &model, item(), EntityId(5)).unwrap();
    let area = create_quantity(&mut tx, &model, quantity(QuantityKind::Area, 1.0)).unwrap();
    let staged = tx.len();
    assert_eq!(
        assign_cost_quantities(&mut tx, &model, cost_item, &[area]),
        Err(CostAuthoringError::AuthoringNotInSchema {
            entity: "IFCCOSTITEM",
            attribute: "CostQuantities",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(tx.len(), staged, "a refusal staged nothing");
}
