//! Lazy loading must be invisible: a strict read that decodes on first
//! access yields exactly the model -- or exactly the error -- of an eager
//! read, and editing, cloning and concurrent access behave the same.

use ifc_model::{Codec, EntityId, Model, Value};
use ifc_step::{ParseOptions, StepCodec, StepReader};
use std::path::{Path, PathBuf};

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test/fixtures");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "ifc") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn eager() -> StepReader {
    StepReader::new(ParseOptions::strict()).eager()
}

/// Everything observable about a read, entities in file order.
fn snapshot(result: &Result<Model, ifc_model::ModelError>) -> String {
    match result {
        Err(error) => format!("ERR {error:?}"),
        Ok(model) => format!(
            "{:?}\n{:?}\n{:?}\n{:?}",
            model.header(),
            model.iter().collect::<Vec<_>>(),
            model.diagnostics(),
            model.type_histogram()
        ),
    }
}

const HEAD: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('d'),'2;1');\n\
FILE_NAME('n','t',('a'),('o'),'p','s','z');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n";
const TAIL: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

fn wrap(data: &str) -> Vec<u8> {
    format!("{HEAD}{data}{TAIL}").into_bytes()
}

#[test]
fn every_fixture_reads_the_same_lazily_and_eagerly() {
    let files = fixtures();
    assert!(
        files.len() > 20,
        "expected a real corpus, found {}",
        files.len()
    );
    let mut lazy_models = 0;
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let lazy = StepCodec.read_bytes(&bytes);
        if let Ok(model) = &lazy {
            // Nothing is decoded until asked for...
            assert_eq!(model.decoded_len(), 0, "{} decoded eagerly", path.display());
            lazy_models += usize::from(!model.is_empty());
        }
        // ...and then every entity decodes to its eager counterpart.
        assert_eq!(
            snapshot(&lazy),
            snapshot(&eager().read_bytes(&bytes)),
            "{}",
            path.display()
        );
    }
    assert!(
        lazy_models > 20,
        "only {lazy_models} fixtures loaded lazily"
    );
}

#[test]
fn conversion_failures_are_the_eager_errors() {
    // Each is valid STEP the IFC record model cannot represent; the lazy
    // validation must reject exactly these, with the eager reader's error.
    let cases = [
        "#1=IFCX(99999999999999999999);\n",
        "#1=IFCX(#99999999999999999999);\n",
        "#99999999999999999999=IFCX(1);\n",
        "#1=(IFCA(1)IFCB(2));\n",
        "#1=IFCX((1,IFCY(99999999999999999999)));\n",
        "#1=IFCX(1);\n#2=IFCX(1,,2);\n",
        "#1=IFCX(1);\njunk\n#2=IFCX(1);\n",
    ];
    for data in cases {
        let bytes = wrap(data);
        let lazy = StepCodec.read_bytes(&bytes);
        assert!(lazy.is_err(), "{data:?} was accepted");
        assert_eq!(
            snapshot(&lazy),
            snapshot(&eager().read_bytes(&bytes)),
            "{data:?}"
        );
    }
    // The boundaries themselves are fine.
    let ok =
        wrap("#18446744073709551615=IFCX(9223372036854775807,-9223372036854775808,1.0E+400);\n");
    let model = StepCodec.read_bytes(&ok).expect("in range");
    assert_eq!(snapshot(&Ok(model)), snapshot(&eager().read_bytes(&ok)));
}

#[test]
fn types_and_ids_are_known_without_decoding() {
    let bytes = wrap("#1=IFCWALL('a');\n#2=ifcwall('b');\n#5=IFCDOOR($);\n");
    let model = StepCodec.read_bytes(&bytes).unwrap();
    assert_eq!(model.len(), 3);
    assert_eq!(model.ids_of_type("IfcWall"), [EntityId(1), EntityId(2)]);
    assert!(model.contains(EntityId(5)) && !model.contains(EntityId(3)));
    assert_eq!(model.next_id(), EntityId(6));
    assert_eq!(model.decoded_len(), 0);
    assert_eq!(
        model.get(EntityId(2)).unwrap().type_name.as_ref(),
        "IFCWALL"
    );
    assert_eq!(model.decoded_len(), 1);
    // A reference stays stable: the second access returns the same entity.
    let first = model.get(EntityId(2)).unwrap() as *const _;
    assert!(std::ptr::eq(first, model.get(EntityId(2)).unwrap()));
}

#[test]
fn duplicate_ids_resolve_as_the_eager_read_does() {
    for data in [
        "#1=IFCWALL('a');\n#1=IFCWALL('b');\n",
        "#1=IFCWALL('a');\n#2=IFCDOOR($);\n#1=IFCDOOR('c');\n",
    ] {
        let bytes = wrap(data);
        assert_eq!(
            snapshot(&StepCodec.read_bytes(&bytes)),
            snapshot(&eager().read_bytes(&bytes)),
            "{data:?}"
        );
    }
}

#[test]
fn edits_on_undecoded_entities_match_edits_on_decoded_ones() {
    let bytes = wrap("#1=IFCWALL('a',#2);\n#2=IFCDOOR($);\n#3=IFCWALL('c');\n");
    let edit = |mut model: Model| {
        model.set_attribute(EntityId(1), 0, Value::Text("x".into()));
        model.retype(EntityId(3), "IFCSLAB");
        model.remove(EntityId(2));
        model.push(ifc_model::Entity::new("IFCNEW", vec![Value::Null]));
        model
    };
    let lazy = edit(StepCodec.read_bytes(&bytes).unwrap());
    let eager = edit(eager().read_bytes(&bytes).unwrap());
    assert_eq!(snapshot(&Ok(lazy.clone())), snapshot(&Ok(eager)));
    assert_eq!(lazy.ids_of_type("IFCSLAB"), [EntityId(3)]);
    assert_eq!(lazy.dangling_references(), [(EntityId(1), EntityId(2))]);
}

#[test]
fn clones_share_the_source_but_not_their_edits() {
    let bytes = wrap("#1=IFCWALL('a');\n#2=IFCWALL('b');\n");
    let original = StepCodec.read_bytes(&bytes).unwrap();
    let mut copy = original.clone();
    copy.set_attribute(EntityId(1), 0, Value::Text("changed".into()));
    assert_eq!(original.get(EntityId(1)).unwrap().text(0), Some("a"));
    assert_eq!(copy.get(EntityId(1)).unwrap().text(0), Some("changed"));
    assert_eq!(copy.get(EntityId(2)).unwrap().text(0), Some("b"));
}

#[test]
fn decode_all_and_concurrent_access_agree_with_eager() {
    let mut data = String::new();
    for id in 1..=5_000 {
        data.push_str(&format!("#{id}=IFCCARTESIANPOINT(({id}.,0.5,-1.E-3));\n"));
    }
    let bytes = wrap(&data);
    let expected = snapshot(&eager().read_bytes(&bytes));

    let model = StepCodec.read_bytes(&bytes).unwrap();
    model.decode_all(8);
    assert_eq!(model.decoded_len(), model.len());
    assert_eq!(snapshot(&Ok(model)), expected);

    // Threads racing to decode the same entities all see one result.
    let model = StepCodec.read_bytes(&bytes).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for id in (1..=5_000).rev() {
                    assert!(model.get(EntityId(id)).is_some());
                }
            });
        }
    });
    assert_eq!(snapshot(&Ok(model)), expected);
}

#[test]
fn every_entry_point_reads_the_same_model() {
    let bytes = wrap("#1=IFCWALL('a',(1,2.5),.T.);\n#2=IFCDOOR(#1,IFCLABEL('x'));\n");
    let expected = snapshot(&eager().read_bytes(&bytes));
    let dir = std::env::temp_dir().join(format!("ifc-lazy-read-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("model.ifc");
    std::fs::write(&path, &bytes).unwrap();

    assert_eq!(snapshot(&StepCodec.read_owned(bytes.clone())), expected);
    assert_eq!(snapshot(&StepCodec.read_path(&path)), expected);
    assert_eq!(
        snapshot(&StepCodec.read_from(&mut bytes.as_slice())),
        expected
    );
    // SAFETY: the file is private to this test and not modified while the
    // model is alive.
    let mapped = unsafe { StepReader::new(ParseOptions::strict()).read_path_mapped(&path) };
    assert_eq!(snapshot(&mapped), expected);
    drop(mapped);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn opting_out_or_recovering_reads_eagerly() {
    let bytes = wrap("#1=IFCWALL('a');\n#2=IFCWALL(1,,2);\n");
    let lenient = StepCodec::lenient().read_bytes(&bytes).unwrap();
    assert_eq!(lenient.decoded_len(), lenient.len());
    assert_eq!(lenient.diagnostics().len(), 1);
    let clean = wrap("#1=IFCWALL('a');\n");
    let eager_model = eager().read_bytes(&clean).unwrap();
    assert_eq!(eager_model.decoded_len(), 1);
}
