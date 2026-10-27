//! Undo and redo through every mutator (#465): with history on, each public
//! `&mut self` mutator is one entry, undo restores the exact bytes and redo
//! the exact result; a scan of `src/` fails when a mutator is added that
//! this table does not exercise.
#![cfg(all(feature = "author", feature = "properties-write", feature = "ifc4"))]

mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use openbim_ifc_binding_core::property_edit::PropertyEdit;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{
    AppendOptions, CopyProductOptions, IfcModel, LengthUnitOptions, Plain, PurgeOptions,
    RemoveProductOptions,
};
use support::{code, create, text};

const UNITS: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCUNITASSIGNMENT((#1,#2));
#4=IFCPROJECT('0000000000000000000005',$,'P',$,$,$,$,$,#3);
#5=IFCCARTESIANPOINT((1000.,2500.,0.));
#6=IFCQUANTITYAREA('Area',$,$,7.5,$);
ENDSEC;
END-ISO-10303-21;
";

const SEED: u128 = 7;

#[derive(Clone, Copy)]
enum Fixture {
    Cascade,
    Selector,
    Properties,
    Units,
}

fn open(name: &str) -> IfcModel {
    let path = format!("{}/../../test/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    IfcModel::open(Path::new(&path)).expect("fixture reads")
}

fn model(fixture: Fixture) -> IfcModel {
    match fixture {
        Fixture::Cascade => open("synthetic-relations/cascade_ifc4.ifc"),
        Fixture::Selector => open("synthetic-relations/selector_ifc4.ifc"),
        Fixture::Properties => open("synthetic-properties/synthetic_properties.ifc"),
        Fixture::Units => IfcModel::parse(UNITS.as_bytes()).expect("parses"),
    }
}

fn label(value: &str) -> Tagged {
    Tagged::Typed {
        type_name: "IFCLABEL".into(),
        value: Box::new(Tagged::Text(value.into())),
    }
}

fn written(model: &IfcModel) -> Vec<u8> {
    model.write().expect("writes")
}

type Act = Box<dyn Fn(&mut IfcModel)>;

struct Case {
    /// The mutator this case exercises.
    name: &'static str,
    fixture: Fixture,
    /// Runs before history is on, so it is not an entry.
    setup: Option<Act>,
    act: Act,
}

fn case(name: &'static str, fixture: Fixture, act: impl Fn(&mut IfcModel) + 'static) -> Case {
    Case {
        name,
        fixture,
        setup: None,
        act: Box::new(act),
    }
}

fn after(mut case: Case, setup: impl Fn(&mut IfcModel) + 'static) -> Case {
    case.setup = Some(Box::new(setup));
    case
}

fn regenerate(dry_run: bool) -> AppendOptions {
    let mut options = AppendOptions::new();
    options.global_ids = "regenerate".into();
    options.dry_run = dry_run;
    options
}

fn cases() -> Vec<Case> {
    use Fixture::*;
    let set_note = |model: &mut IfcModel| {
        model
            .set_property(31, "Custom", "Note", label("one"), None)
            .expect("sets");
    };
    let cascade_source = || open("synthetic-relations/cascade_ifc4.ifc");
    vec![
        case("set_header", Cascade, |m| {
            let mut header = m.header();
            header.name = "renamed.ifc".into();
            m.set_header(header);
        }),
        case("set_attribute", Cascade, |m| {
            m.set_attribute(20, 2, Tagged::Text("Renamed".into()))
                .expect("sets");
        }),
        case("add", Cascade, |m| {
            m.add("IFCLABEL", vec![Tagged::Text("x".into())])
                .expect("adds");
        }),
        case("remove", Cascade, |m| {
            m.remove(80).expect("removes");
        }),
        case("set_attribute_by_name", Cascade, |m| {
            m.set_attribute_by_name(20, "Name", Tagged::Text("Renamed".into()))
                .expect("sets");
        }),
        case("set_attribute_by_name_plain", Cascade, |m| {
            m.set_attribute_by_name_plain(20, "Name", Plain::Text("Renamed".into()))
                .expect("sets");
        }),
        case("set_properties", Properties, |m| {
            m.set_properties(vec![PropertyEdit::set(31, "Custom", "Note", label("one"))])
                .expect("sets");
        }),
        case("set_properties_seeded", Properties, |m| {
            m.set_properties_seeded(
                vec![PropertyEdit::set(31, "Custom", "Note", label("one"))],
                SEED,
            )
            .expect("sets");
        }),
        case("set_property", Properties, set_note),
        case("set_property_seeded", Properties, |m| {
            m.set_property_seeded(31, "Custom", "Note", label("one"), None, SEED)
                .expect("sets");
        }),
        after(
            case("remove_property", Properties, |m| {
                m.remove_property(31, "Custom", "Note").expect("removes");
            }),
            set_note,
        ),
        after(
            case("remove_property_seeded", Properties, |m| {
                m.remove_property_seeded(31, "Custom", "Note", SEED)
                    .expect("removes");
            }),
            set_note,
        ),
        case("import_table", Selector, |m| {
            let import = m
                .import_table(b"id,Name\r\n17,Renamed\r\n", "csv", Some("id"), "", false)
                .expect("imports");
            assert!(import.applied, "{import:?}");
        }),
        case("import_table_seeded", Selector, |m| {
            let import = m
                .import_table_seeded(
                    b"id,Name\r\n17,Renamed\r\n",
                    "csv",
                    Some("id"),
                    "",
                    false,
                    SEED,
                )
                .expect("imports");
            assert!(import.applied, "{import:?}");
        }),
        case("copy_product", Cascade, |m| {
            m.copy_product(20, CopyProductOptions::new())
                .expect("copies");
        }),
        case("copy_product_seeded", Cascade, |m| {
            m.copy_product_seeded(20, CopyProductOptions::new(), SEED)
                .expect("copies");
        }),
        case("append", Cascade, move |m| {
            m.append(&cascade_source(), &[20], &regenerate(false))
                .expect("appends");
        }),
        case("append_seeded", Cascade, move |m| {
            m.append_seeded(&cascade_source(), &[20], &regenerate(false), SEED)
                .expect("appends");
        }),
        case("set_value", Selector, |m| {
            m.set_value(17, "type.Name", Plain::Text("WT-300".into()))
                .expect("sets");
        }),
        case("set_value_seeded", Selector, |m| {
            m.set_value_seeded(17, "type.Name", Plain::Text("WT-300".into()), SEED)
                .expect("sets");
        }),
        case("convert_length_unit", Units, |m| {
            let report = m
                .convert_length_unit("METRE", LengthUnitOptions::new())
                .expect("converts");
            assert!(report.applied);
        }),
        case("remove_product", Cascade, |m| {
            m.remove_product(100, RemoveProductOptions::new())
                .expect("removes");
        }),
        case("purge", Cascade, |m| {
            let report = m.purge(PurgeOptions::new()).expect("purges");
            assert!(report.applied);
        }),
        case("remove_representation", Cascade, |m| {
            m.remove_representation(25, false).expect("removes");
        }),
        case("author", Cascade, |m| {
            m.author(vec![create(
                "IfcBuildingElementProxy",
                &[("Name", text("Proxy"))],
            )])
            .expect("authors");
        }),
        case("author_seeded", Cascade, |m| {
            m.author_seeded(
                vec![create(
                    "IfcBuildingElementProxy",
                    &[("Name", text("Proxy"))],
                )],
                SEED,
            )
            .expect("authors");
        }),
        case("create_entity", Cascade, |m| {
            m.create_entity(
                "IfcBuildingElementProxy",
                vec![("Name".into(), text("Proxy"))],
            )
            .expect("creates");
        }),
        case("remove_with_relationships", Cascade, |m| {
            m.remove_with_relationships(20).expect("removes");
        }),
    ]
}

/// Mutators that only drive the history itself.
const CONTROLS: &[&str] = &["set_history_limit", "undo", "redo", "clear_history"];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("reads src") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Whether the first parameter is `&mut self`, `&'a mut self` or
/// `self: &mut Self`.
fn is_mut_receiver(first: &str) -> bool {
    let spaced = first.replace('&', " & ").replace(':', " : ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    match tokens.as_slice() {
        ["&", "mut", "self"] | ["self", ":", "&", "mut", "Self"] => true,
        ["&", lifetime, "mut", "self"] | ["self", ":", "&", lifetime, "mut", "Self"] => {
            lifetime.starts_with('\'')
        }
        _ => false,
    }
}

/// `(name, first parameter)` of every `fn` whose text follows one of the
/// `marker` prefixes.
fn functions(code: &str, marker: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for part in code.split(marker).skip(1) {
        let name: String = part
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let mut rest = &part[name.len()..];
        if rest.starts_with('<') {
            let mut depth = 0;
            let end = rest
                .char_indices()
                .find(|(_, c)| {
                    depth += i32::from(*c == '<') - i32::from(*c == '>');
                    depth == 0
                })
                .map_or(rest.len(), |(at, _)| at + 1);
            rest = &rest[end..];
        }
        let Some(open) = rest.trim_start().strip_prefix('(') else {
            continue;
        };
        let first = open.split([',', ')']).next().unwrap_or_default();
        found.push((name, first.to_owned()));
    }
    found
}

fn source_files() -> Vec<String> {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files
        .iter()
        .map(|file| {
            let source = std::fs::read_to_string(file).expect("reads");
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

/// Every `pub fn` of `src/` whose first parameter is a `&mut self` receiver.
fn mutators() -> BTreeSet<String> {
    source_files()
        .iter()
        .flat_map(|code| functions(code, "pub fn "))
        .filter(|(_, first)| is_mut_receiver(first))
        .map(|(name, _)| name)
        .collect()
}

/// The `&mut self` functions of every `impl ... for IfcModel` block: a trait
/// impl is a mutator the `pub fn` scan cannot see.
fn trait_impl_mutators() -> Vec<String> {
    let mut found = Vec::new();
    for code in source_files() {
        let mut from = 0;
        while let Some(at) = code[from..].find(" for IfcModel") {
            let start = from + at;
            from = start + 1;
            let Some(open) = code[start..].find('{') else {
                continue;
            };
            let body = &code[start + open..];
            let mut depth = 0;
            let end = body
                .char_indices()
                .find(|(_, c)| {
                    depth += i32::from(*c == '{') - i32::from(*c == '}');
                    depth == 0
                })
                .map_or(body.len(), |(at, _)| at);
            found.extend(
                functions(&body[..end], "fn ")
                    .into_iter()
                    .filter(|(_, first)| is_mut_receiver(first))
                    .map(|(name, _)| name),
            );
        }
    }
    found
}

#[test]
fn receivers_are_recognised_in_every_spelling() {
    for yes in [
        "&mut self",
        "& mut  self",
        "&'a mut self",
        "self: &mut Self",
        "self : &'a mut Self",
    ] {
        assert!(is_mut_receiver(yes), "{yes}");
    }
    for no in [
        "&self",
        "self",
        "mut self",
        "&'a self",
        "x: &mut IfcModel",
        "",
    ] {
        assert!(!is_mut_receiver(no), "{no}");
    }
}

#[test]
fn no_trait_impl_on_the_model_mutates_it() {
    assert_eq!(trait_impl_mutators(), Vec::<String>::new());
}

#[test]
fn every_mutator_is_covered() {
    let cases = cases();
    let covered: BTreeSet<String> = cases
        .iter()
        .map(|case| case.name.to_owned())
        .chain(CONTROLS.iter().map(|name| (*name).to_owned()))
        .collect();
    assert_eq!(
        covered.len(),
        cases.len() + CONTROLS.len(),
        "a name repeats"
    );
    assert_eq!(
        mutators(),
        covered,
        "a `&mut self` method of the binding core is not in this table: wrap it in \
         `tracked` and add a case"
    );
}

#[test]
fn every_mutator_undoes_and_redoes_exactly() {
    for case in cases() {
        let mut m = model(case.fixture);
        if let Some(setup) = &case.setup {
            setup(&mut m);
        }
        m.set_history_limit(8);
        let (b0, ids0) = (written(&m), m.ids());
        (case.act)(&mut m);
        let (b1, ids1) = (written(&m), m.ids());
        assert_ne!(b0, b1, "{} changed nothing", case.name);
        assert_eq!(
            (m.history().undo, m.history().redo),
            (1, 0),
            "{} is not one entry",
            case.name
        );

        m.undo().unwrap_or_else(|e| panic!("{}: {e}", case.name));
        assert_eq!(written(&m), b0, "{}: undo", case.name);
        assert_eq!(m.ids(), ids0, "{}: undo ids", case.name);
        assert_eq!((m.history().undo, m.history().redo), (0, 1));

        m.redo().unwrap_or_else(|e| panic!("{}: {e}", case.name));
        assert_eq!(written(&m), b1, "{}: redo", case.name);
        assert_eq!(m.ids(), ids1, "{}: redo ids", case.name);
    }
}

#[test]
fn history_is_off_by_default() {
    let mut m = model(Fixture::Cascade);
    m.remove(80).unwrap();
    assert_eq!(m.history().limit, 0);
    assert_eq!(code(m.undo()), "history-empty");
    assert_eq!(code(m.redo()), "history-empty");
    assert_eq!(code(m.peek_undo()), "history-empty");
}

#[test]
fn a_nested_batch_is_one_entry() {
    let mut m = model(Fixture::Cascade);
    m.set_history_limit(8);
    m.create_entity(
        "IfcBuildingElementProxy",
        vec![("Name".into(), text("Proxy"))],
    )
    .unwrap();
    assert_eq!(m.history().undo, 1);
}

#[test]
fn refusals_dry_runs_and_peeks_record_nothing() {
    let mut m = model(Fixture::Cascade);
    m.set_history_limit(8);
    assert_eq!(code(m.remove(9999)), "missing-entity");
    let mut dry = RemoveProductOptions::new();
    dry.dry_run = true;
    m.remove_product(100, dry).unwrap();
    assert_eq!(m.history().undo, 0);

    m.remove(80).unwrap();
    let before = written(&m);
    let peeked = m.peek_undo().unwrap();
    assert_eq!(peeked.created, [80]);
    assert_eq!(written(&m), before);
    assert_eq!(m.history().undo, 1);
    let undone = m.undo().unwrap();
    assert_eq!(undone, peeked);
    assert_eq!(code(m.peek_undo()), "history-empty");
    assert_eq!(m.peek_redo().unwrap().removed, [80]);

    // A refused call after an undo leaves the redo entry alone.
    assert_eq!(m.history().redo, 1);
    assert_eq!(code(m.remove(9999)), "missing-entity");
    assert_eq!((m.history().undo, m.history().redo), (0, 1));
    m.redo().unwrap();
}

#[test]
fn dry_runs_of_every_planning_mutator_record_nothing() {
    let mut m = model(Fixture::Cascade);
    m.set_history_limit(8);
    let before = written(&m);
    let mut copy = CopyProductOptions::new();
    copy.dry_run = true;
    assert!(!m.copy_product(20, copy).unwrap().applied);
    let source = model(Fixture::Cascade);
    assert!(!m.append(&source, &[20], &regenerate(true)).unwrap().applied);
    assert_eq!(m.history().undo, 0);
    assert_eq!(written(&m), before);

    let mut selector = model(Fixture::Selector);
    selector.set_history_limit(8);
    let import = selector
        .import_table(b"id,Name\r\n17,Renamed\r\n", "csv", Some("id"), "", true)
        .unwrap();
    assert!(!import.applied && !import.changes.is_empty());
    assert_eq!(selector.history().undo, 0);

    let mut units = model(Fixture::Units);
    units.set_history_limit(8);
    let mut dry = LengthUnitOptions::new();
    dry.dry_run = true;
    assert!(!units.convert_length_unit("METRE", dry).unwrap().applied);
    assert_eq!(units.history().undo, 0);
}

#[test]
fn clear_history_and_a_lower_limit() {
    let mut m = model(Fixture::Cascade);
    m.set_history_limit(2);
    for id in [80, 81, 82] {
        m.remove(id).unwrap();
    }
    assert_eq!(m.history().undo, 2, "the oldest entry is dropped");
    m.undo().unwrap();
    assert_eq!((m.history().undo, m.history().redo), (1, 1));
    m.set_history_limit(1);
    assert_eq!((m.history().undo, m.history().redo), (1, 1));
    m.clear_history();
    assert_eq!(
        (m.history().limit, m.history().undo, m.history().redo),
        (1, 0, 0)
    );
    m.set_history_limit(0);
    assert_eq!(m.history().limit, 0);
}
