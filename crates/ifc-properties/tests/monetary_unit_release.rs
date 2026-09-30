//! `IfcMonetaryUnit.Currency` is read and written in the model's release
//! (#232).
//!
//! From the EXPRESS sources in `references/ifc-spec/`: IFC2X3 TC1 declares
//! `Currency : IfcCurrencyEnum`; IFC4 ADD2 TC1, IFC4X1, IFC4X2 and IFC4X3
//! ADD2 declare `Currency : IfcLabel`. The first test pins the bundled
//! tables the writer binds to against that EXPRESS; the rest author a unit,
//! write the model to STEP text, read it back and read the currency.

use ifc_model::{Codec, Model, Transaction, Value};
use ifc_properties::{
    create_monetary_unit, project_units, unit, MonetaryUnitDraft, PropertyError, UnitKind,
};
use ifc_schema::{for_version, Schema, SchemaVersion, TypeKind};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn spec(rel: &str) -> Option<Schema> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(root) = [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|path| crate_dir.join(path))
    .find(|path| path.is_dir()) else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
             run scripts/fetch-ifc-schemas.sh"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return None;
    };
    let bytes = std::fs::read(root.join(rel)).expect("read the reference schema");
    Some(Schema::from_express_bytes(&bytes))
}

/// The declared type of `IfcMonetaryUnit.Currency` in `schema`.
fn currency_type(schema: &Schema) -> String {
    schema
        .attributes("IFCMONETARYUNIT")
        .into_iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case("Currency"))
        .expect("IfcMonetaryUnit declares Currency")
        .type_name
        .clone()
}

#[test]
fn bundled_currency_types_match_the_reference_express() {
    for (version, rel, declared) in [
        (
            SchemaVersion::Ifc2x3,
            "ifc2x3-tc1/IFC2X3_TC1.exp",
            "IfcCurrencyEnum",
        ),
        (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp", "IfcLabel"),
        (
            SchemaVersion::Ifc4x3,
            "ifc4x3-add2/IFC4X3_ADD2.exp",
            "IfcLabel",
        ),
    ] {
        let Some(reference) = spec(rel) else {
            return;
        };
        let bundled = for_version(version).expect("bundled table");
        assert!(currency_type(&reference).eq_ignore_ascii_case(declared));
        assert!(currency_type(bundled).eq_ignore_ascii_case(declared));
        let members = |schema: &Schema| match schema.type_def(declared).map(|t| &t.kind) {
            Some(TypeKind::Enumeration(members)) => members.clone(),
            _ => Vec::new(),
        };
        assert_eq!(members(bundled), members(&reference), "{version:?}");
    }
}

fn declaring(schema: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema.to_owned()];
    model
}

/// Author one monetary unit in `schema`, write STEP text, read it back, and
/// return the text and the reread model.
fn round_trip(schema: &str, currency: &str) -> (String, Model) {
    let mut model = declaring(schema);
    let mut tx = Transaction::new(&model);
    create_monetary_unit(&mut tx, &model, MonetaryUnitDraft::new(currency))
        .expect("the currency is valid in this release");
    tx.commit(&mut model).expect("commits");
    let bytes = StepCodec.write_bytes(&model).expect("writes");
    let text = String::from_utf8(bytes).expect("STEP text is ASCII");
    let reread = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    (text, reread)
}

fn currency(model: &Model) -> Option<String> {
    let (id, _) = model
        .of_type("IFCMONETARYUNIT")
        .next()
        .expect("one monetary unit");
    match unit(model, id) {
        Some(UnitKind::Monetary { currency }) => currency.map(|c| c.to_string()),
        other => panic!("not a monetary unit: {other:?}"),
    }
}

#[test]
fn ifc2x3_writes_and_reads_the_currency_enum() {
    // Matched ignoring case, written as the schema's enumerator.
    let (text, model) = round_trip("IFC2X3", "eur");
    assert!(text.contains("IFCMONETARYUNIT(.EUR.)"), "{text}");
    assert_eq!(
        model
            .get(model.ids_of_type("IFCMONETARYUNIT")[0])
            .unwrap()
            .attributes,
        vec![Value::Enum("EUR".into())]
    );
    assert_eq!(currency(&model).as_deref(), Some("EUR"));
}

#[test]
fn ifc4_and_ifc4x3_write_and_read_the_currency_label() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let (text, model) = round_trip(schema, "EUR");
        assert!(text.contains("IFCMONETARYUNIT('EUR')"), "{schema}: {text}");
        assert_eq!(currency(&model).as_deref(), Some("EUR"), "{schema}");
        // A label is free text there: a code IfcCurrencyEnum never listed
        // is still a currency.
        let (_, model) = round_trip(schema, "XBT");
        assert_eq!(currency(&model).as_deref(), Some("XBT"), "{schema}");
    }
}

#[test]
fn ifc2x3_refuses_a_currency_the_enum_does_not_list() {
    let model = declaring("IFC2X3");
    let mut tx = Transaction::new(&model);
    let refused = create_monetary_unit(&mut tx, &model, MonetaryUnitDraft::new("XBT"));
    assert!(
        matches!(
            refused,
            Err(PropertyError::AuthoringInvalid {
                entity: "IFCMONETARYUNIT",
                attribute: "Currency",
                ..
            })
        ),
        "{refused:?}"
    );
    assert!(tx.is_empty(), "a refused currency stages nothing");
}

#[test]
fn blank_currencies_and_unbound_headers_are_refused() {
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let model = declaring(schema);
        let mut tx = Transaction::new(&model);
        assert!(matches!(
            create_monetary_unit(&mut tx, &model, MonetaryUnitDraft::new("  ")),
            Err(PropertyError::AuthoringInvalid { .. })
        ));
        assert!(tx.is_empty());
    }
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC2X3".into(), "IFC4".into()];
    let mut tx = Transaction::new(&model);
    assert!(matches!(
        create_monetary_unit(&mut tx, &model, MonetaryUnitDraft::new("EUR")),
        Err(PropertyError::MultipleSchemas { schemas: 2 })
    ));
    let model = declaring("IFC4X2");
    let mut tx = Transaction::new(&model);
    assert!(matches!(
        create_monetary_unit(&mut tx, &model, MonetaryUnitDraft::new("EUR")),
        Err(PropertyError::UnsupportedSchema { .. })
    ));
    assert!(tx.is_empty());
}

/// The reader takes both forms whatever the header says: a file converted
/// between releases can carry either.
#[test]
fn the_reader_accepts_the_enum_and_the_label_form() {
    let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\n\
         DATA;\n#1=IFCMONETARYUNIT(.GBP.);\n#2=IFCMONETARYUNIT('EUR');\n\
         #3=IFCUNITASSIGNMENT((#1,#2));\nENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    let currencies: Vec<_> = project_units(&model)
        .into_iter()
        .map(|(_, kind)| match kind {
            UnitKind::Monetary { currency } => currency.map(|c| c.to_string()),
            other => panic!("not a monetary unit: {other:?}"),
        })
        .collect();
    assert_eq!(
        currencies,
        vec![Some("GBP".to_owned()), Some("EUR".to_owned())]
    );
}
