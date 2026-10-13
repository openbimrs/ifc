//! ifcZIP (#450): STEP and ifcXML round trips through the archive, and
//! every refusal by its own name.

#![cfg(all(
    feature = "zip",
    feature = "step",
    feature = "ifcxml",
    feature = "ifc4"
))]

use std::sync::Arc;

use ifc::ifczip::{self, Budget, IfcZipCodec, IfcZipError, InnerFormat, Limit};
use ifc::{Codec, Model, ModelError, StepCodec, XmlCodec, XmlProfile};

const FIXTURE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/fixtures/synthetic-bindings/binding_geometry.ifc"
));

fn model() -> Model {
    StepCodec.read_bytes(FIXTURE).expect("the fixture reads")
}

fn ifc4() -> Arc<ifc::Schema> {
    Arc::new(
        ifc::schema::for_version(ifc::SchemaVersion::Ifc4)
            .unwrap()
            .clone(),
    )
}

/// An archive of `(name, contents)` entries, Stored or Deflated.
fn archive(entries: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
    use std::io::Write;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    for (name, contents) in entries {
        if name.ends_with('/') {
            writer.add_directory(*name, options).unwrap();
        } else {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}

fn stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    archive(entries, zip::CompressionMethod::Stored)
}

/// Overwrite the little-endian `u16` at `offset` of every local and central
/// header of `bytes` (header offsets: flags 6 and 8, method 8 and 10).
fn patch_headers(bytes: &mut [u8], local: usize, central: usize, value: u16) {
    let mut patched = 0;
    for at in 0..bytes.len().saturating_sub(4) {
        let offset = match &bytes[at..at + 4] {
            b"PK\x03\x04" => local,
            b"PK\x01\x02" => central,
            _ => continue,
        };
        bytes[at + offset..at + offset + 2].copy_from_slice(&value.to_le_bytes());
        patched += 1;
    }
    assert_eq!(patched, 2, "one local and one central header");
}

fn refusal(bytes: &[u8]) -> IfcZipError {
    ifczip::read(bytes, &Budget::DEFAULT).expect_err("refused")
}

#[test]
fn step_round_trips_byte_identical() {
    let model = model();
    let step = StepCodec.write_bytes(&model).unwrap();
    let zipped = IfcZipCodec::default().write_bytes(&model).unwrap();
    assert!(ifczip::is_zip(&zipped));

    let entry = ifczip::extract(&zipped, &Budget::DEFAULT).unwrap();
    assert_eq!(entry.name, "model.ifc");
    assert_eq!(entry.format, InnerFormat::Step);
    assert!(entry.bytes == step, "the archive holds the STEP bytes");

    let back = IfcZipCodec::default().read_bytes(&zipped).unwrap();
    assert!(
        StepCodec.write_bytes(&back).unwrap() == step,
        "STEP changed"
    );
    // A packed file reads back to the bytes it was packed from.
    let packed = ifczip::pack("Model.IFC", FIXTURE).unwrap();
    assert!(ifczip::extract(&packed, &Budget::DEFAULT).unwrap().bytes == FIXTURE);
    assert!(zipped.len() < step.len(), "Deflate compresses");
}

#[test]
fn ifcxml_round_trips_byte_identical_in_both_layouts() {
    let model = model();
    let codecs = [
        XmlCodec::default(),
        XmlCodec::with_schema(ifc4()),
        XmlCodec::xsd(ifc4(), XmlProfile::Ifc4Add2Tc1),
    ];
    for codec in codecs {
        let xml = codec.write_bytes(&model).unwrap();
        let zipped = ifczip::write(&model, &codec, "model.ifcXML").unwrap();
        let entry = ifczip::extract(&zipped, &Budget::DEFAULT).unwrap();
        assert_eq!(entry.format, InnerFormat::IfcXml);
        assert!(entry.bytes == xml, "the archive holds the ifcXML bytes");
        let back = ifczip::read(&zipped, &Budget::DEFAULT)
            .unwrap_or_else(|error| panic!("{:?}: {error}", codec.layout()));
        // The archive reads as the document does on its own: the layout
        // and tables chosen from the root are the ones it was written with.
        // (The XSD layout renumbers entities, so its document is compared
        // after one read; the native layouts round-trip exactly.)
        let direct = codec.read_bytes(&xml).unwrap();
        let rewritten = codec.write_bytes(&back).unwrap();
        assert!(
            rewritten == codec.write_bytes(&direct).unwrap(),
            "{:?} layout: the archive read differently",
            codec.layout()
        );
        if codec.layout() == ifc::XmlLayout::Native {
            assert!(rewritten == xml, "native layout: the document changed");
        }
        assert_eq!(back.len(), model.len());
    }
}

#[test]
fn writing_is_reproducible() {
    let model = model();
    let codec = IfcZipCodec::default();
    assert!(codec.write_bytes(&model).unwrap() == codec.write_bytes(&model).unwrap());
}

#[test]
fn read_path_and_the_codec_list_take_ifczip() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ifczip-450");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("read_path.ifczip");
    std::fs::write(&path, IfcZipCodec::default().write_bytes(&model()).unwrap()).unwrap();
    let read = ifc::read_path(&path).unwrap();
    assert_eq!(read.len(), model().len());
    assert!(ifc::codecs().iter().any(|codec| codec.name() == "ifcZIP"));
    assert!(ifc::compiled_features().contains(&"zip"));
}

#[test]
fn an_archive_without_a_model_is_refused() {
    assert_eq!(
        refusal(&stored(&[])),
        IfcZipError::NoModel { entries: vec![] }
    );
    assert_eq!(
        refusal(&stored(&[("readme.txt", b"hello"), ("docs/", b"")])),
        IfcZipError::NoModel {
            entries: vec!["readme.txt".to_owned()]
        }
    );
    // A model in a subdirectory is not the archive's model (CV-2x3-154).
    assert!(matches!(
        refusal(&stored(&[("sub/model.ifc", FIXTURE)])),
        IfcZipError::NoModel { .. }
    ));
}

#[test]
fn several_models_are_refused() {
    let bytes = stored(&[
        ("a.ifc", FIXTURE),
        ("b.ifcXML", b"<ifcXML/>"),
        ("links/c.ifc", FIXTURE),
    ]);
    assert_eq!(
        refusal(&bytes),
        IfcZipError::SeveralModels {
            names: vec!["a.ifc".to_owned(), "b.ifcXML".to_owned()]
        }
    );
}

#[test]
fn an_entry_in_another_format_is_refused() {
    let error = refusal(&stored(&[("model.ifc", b"{\"not\": \"ifc\"}")]));
    assert_eq!(
        error,
        IfcZipError::UnknownInnerFormat {
            name: "model.ifc".to_owned()
        }
    );
    // Writing refuses what reading would.
    assert!(matches!(
        ifczip::pack("model.ifc", b"plain text"),
        Err(IfcZipError::UnknownInnerFormat { .. })
    ));
    assert!(matches!(
        ifczip::pack("dir/model.ifc", FIXTURE),
        Err(IfcZipError::InvalidEntryName(_))
    ));
    assert!(matches!(
        IfcZipCodec::wrapping(StepCodec, "model.step"),
        Err(IfcZipError::InvalidEntryName(_))
    ));
}

#[test]
fn an_encrypted_entry_is_refused() {
    let mut bytes = stored(&[("model.ifc", FIXTURE)]);
    patch_headers(&mut bytes, 6, 8, 1); // general purpose flag bit 0
    assert_eq!(
        refusal(&bytes),
        IfcZipError::Encrypted {
            name: "model.ifc".to_owned()
        }
    );
}

#[test]
fn compression_other_than_stored_or_deflate_is_refused() {
    for (method, name) in [(9u16, "Deflate64"), (12, "BZIP2"), (93, "Zstd")] {
        let mut bytes = stored(&[("model.ifc", FIXTURE)]);
        patch_headers(&mut bytes, 8, 10, method);
        match refusal(&bytes) {
            IfcZipError::UnsupportedCompression {
                name: entry,
                method: shown,
            } => {
                assert_eq!(entry, "model.ifc");
                assert!(
                    shown
                        .to_ascii_lowercase()
                        .contains(&name.to_ascii_lowercase())
                        && shown.contains(&method.to_string()),
                    "{shown}"
                );
            }
            other => panic!("method {method}: {other:?}"),
        }
    }
    // Both methods ifcZIP allows read.
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let bytes = archive(&[("model.ifc", FIXTURE)], method);
        assert_eq!(
            ifczip::read(&bytes, &Budget::DEFAULT).unwrap().len(),
            model().len()
        );
    }
}

#[test]
fn a_zip_bomb_is_refused_by_the_ratio_budget() {
    // 64 MiB of white space in a STEP file deflates about 1000 to 1.
    let mut bomb = b"ISO-10303-21;\n".to_vec();
    bomb.resize(64 << 20, b' ');
    let bytes = archive(&[("model.ifc", &bomb)], zip::CompressionMethod::Deflated);
    assert!(bytes.len() < 1 << 20);
    match refusal(&bytes) {
        IfcZipError::BudgetExceeded {
            name,
            limit: Limit::Ratio { ratio, compressed },
        } => {
            assert_eq!(name, "model.ifc");
            assert_eq!(ratio, 200);
            assert!(compressed < 1 << 20);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_absolute_budget_and_an_understated_size_are_refused() {
    let zipped = IfcZipCodec::default().write_bytes(&model()).unwrap();
    let small = Budget::DEFAULT.max_bytes(1000);
    assert_eq!(
        ifczip::read(&zipped, &small).expect_err("over budget"),
        IfcZipError::BudgetExceeded {
            name: "model.ifc".to_owned(),
            limit: Limit::Bytes(1000)
        }
    );
    // An archive that declares a size within budget but inflates past it is
    // refused while inflating, never held whole.
    let mut lying = archive(&[("model.ifc", FIXTURE)], zip::CompressionMethod::Deflated);
    patch_headers(&mut lying, 22, 24, 10); // the uncompressed size's low half
    let budget = Budget::DEFAULT.max_bytes(100);
    assert!(matches!(
        ifczip::read(&lying, &budget),
        Err(IfcZipError::BudgetExceeded { .. } | IfcZipError::Malformed(_))
    ));
    assert!(ifczip::read(&zipped, &Budget::DEFAULT).is_ok());
}

#[test]
fn a_damaged_archive_is_malformed() {
    assert!(matches!(
        refusal(b"PK\x03\x04 this is not an archive"),
        IfcZipError::Malformed(_)
    ));
    // Through the codec interface, an archive refusal names ifcZIP.
    match IfcZipCodec::default().read_bytes(&stored(&[])) {
        Err(ModelError::WrongFormat { expected, detail }) => {
            assert_eq!(expected, "ifcZIP");
            assert!(detail.contains("no .ifc or .ifcXML entry"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_inner_codec_refusal_names_the_entry() {
    let bytes = stored(&[("model.ifc", b"ISO-10303-21;\nHEADER;\nnot step")]);
    match refusal(&bytes) {
        IfcZipError::Read { name, .. } => assert_eq!(name, "model.ifc"),
        other => panic!("{other:?}"),
    }
}
