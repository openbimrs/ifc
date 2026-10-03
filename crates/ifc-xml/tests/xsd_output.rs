//! Output against the official release XSDs, opt in.
//!
//! # What the XSD layout claims
//!
//! [`XmlCodec::xsd`] writes the buildingSMART XSD configuration. Its claim
//! is the strongest: every document it writes validates against the
//! release XSD. The check writes the hand-built construct models of
//! `support/xsd_models.rs` and every IFC4 fixture (a model the writer
//! refuses writes no document; `xsd_corpus_write.rs` pins those), and
//! `xmllint` must report every document valid, with no validity error at
//! all. For IFC4X3 ADD2 the documents must be well-formed and in the XSD's
//! target namespace, since that XSD does not compile (below).
//!
//! # What the strict profile claims
//!
//! [`XmlCodec::with_schema_and_profile`] writes this crate's own lossless
//! layout (upper-case STEP type names, `i<n>` ids, `kind` child elements,
//! a `schema` attribute on the root, a header mirroring STEP's) under the
//! release's exact namespace and schema token. It does **not** write the
//! buildingSMART XSD configuration (that is [`XmlCodec::xsd`]), so its
//! output is not valid against the release XSD and does not claim to be. What it claims is that
//! the output is well-formed XML whose root is the XSD's `ifcXML` element in
//! the XSD's target namespace. This check validates exactly that with a
//! W3C XML Schema validator, and pins the ways the output departs from the
//! XSD so the documentation cannot drift from the code:
//!
//! - every IFC4 and IFC4X3 fixture under `test/fixtures` is written with the
//!   strict profile of its release;
//! - `xmllint --noout` must accept every document (well-formed XML);
//! - the XSD's `targetNamespace` must be [`XmlProfile::namespace`];
//! - `xmllint --schema` must accept a minimal XSD-configuration control
//!   document, which [`XmlCodec::xsd`] must read too: without it, a refusal
//!   would prove nothing about the validator;
//! - every validity error on a written document must be one of
//!   [`DEVIATIONS`], and a document with entities must show the entity
//!   deviation. Any other error (a wrong namespace or root, a malformed
//!   document, an unexpected type error) fails the check.
//!
//! `xmllint` reports the first unexpected entity element of the root and
//! then stops checking the root's content, so beyond the envelope (root,
//! namespace, root attributes, header) the check proves well-formedness
//! only. That is all the strict profile claims.
//!
//! The published `IFC4X3_ADD2.xsd` is not a valid XML Schema: libxml2 and
//! Xerces both refuse to compile it (`maxOccurs="?"`, and the complex types
//! `IfcBinary` and `IfcCompoundPlaneAngleMeasure` used as simple types). No
//! conforming validator can check a document against it, so for that
//! release the check asserts the compile failure ([`UNCOMPILABLE`]) and
//! still runs the well-formedness and namespace checks. When a corrected
//! XSD compiles, the check fails until the entry is removed.
//!
//! # Running it
//!
//! It is `#[ignore]`d, so normal builds and the gate never need the XSDs, a
//! network or `xmllint`. Once enabled, a missing XSD, a missing validator
//! or an empty corpus is a failure, never a skip:
//!
//! ```text
//! scripts/fetch-ifc-schemas.sh            # fetches IFC4.xsd, IFC4X3_ADD2.xsd
//! sudo apt-get install libxml2-utils      # provides xmllint
//! cargo test -p ifc-xml --test xsd_output -- --ignored --nocapture
//! ```
//!
//! `XMLLINT=<path>` selects another `xmllint` binary; without root, extract
//! one from the package (`apt-get download libxml2-utils`, then
//! `dpkg-deb -x libxml2-utils_*.deb <dir>`) and point `XMLLINT` at
//! `<dir>/usr/bin/xmllint`. The written documents stay under
//! `target/tmp/ifc-xml-xsd-output/` (strict) and
//! `target/tmp/ifc-xml-xsd-writer-output/` (XSD layout) for inspection.
//!
//! `xmllint` (libxml2) is the validator rather than a Rust crate: it is the
//! reference W3C XML Schema 1.0 implementation on every Linux distribution,
//! and a Rust binding to it (`libxml`) would make every test build need the
//! libxml2 headers. The pure-Rust XSD validators on crates.io are pre-1.0
//! and unproven against schemas of this size.
#![cfg(feature = "schema")]

#[path = "support/xsd_models.rs"]
#[allow(dead_code)]
mod support;

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use ifc_xml::{XmlCodec, XmlProfile};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

/// The corpus root, relative to this crate.
const FIXTURE_ROOT: &str = "../../test/fixtures";

/// The ways strict output departs from the release XSD, as
/// `(name, message pattern, reason)`. A validity error matches when its
/// message contains every `|`-separated part of the pattern.
const DEVIATIONS: &[(&str, &str, &str)] = &[
    (
        "root schema attribute",
        "Element '{NS}ifcXML', attribute 'schema'|The attribute 'schema' is not allowed",
        "the native layout carries the schema token on the root; the XSD \
         configuration has no such attribute",
    ),
    (
        "entity element",
        "Element '{NS}IFC|This element is not expected",
        "the native layout names entity elements by their upper-case STEP \
         type name with native attribute encodings; the XSD declares \
         `IfcWall` and its configuration",
    ),
    (
        "header layout",
        "HEADER|This element is not expected",
        "the native header mirrors STEP's FILE_NAME and FILE_DESCRIPTION in \
         its own order, repeats `author` and `organization`, and writes \
         `description`, which the XSD header does not declare",
    ),
];

/// Header children the native writer emits.
const HEADER_ELEMENTS: &[&str] = &[
    "name",
    "time_stamp",
    "preprocessor_version",
    "originating_system",
    "authorization",
    "author",
    "organization",
    "description",
];

/// Release XSDs that no conforming validator compiles, with the reason.
const UNCOMPILABLE: &[(XmlProfile, &str)] = &[(
    XmlProfile::Ifc4x3Add2,
    "IFC4X3_ADD2.xsd declares maxOccurs=\"?\" and uses the complex types \
     IfcBinary and IfcCompoundPlaneAngleMeasure as simple types",
)];

/// Releases checked, with their XSD under `references/ifc-spec` and the
/// fewest fixtures that may declare them.
const RELEASES: &[(XmlProfile, &str, &str, usize)] = &[
    (XmlProfile::Ifc4Add2Tc1, "ifc4-add2-tc1", "IFC4.xsd", 30),
    (XmlProfile::Ifc4x3Add2, "ifc4x3-add2", "IFC4X3_ADD2.xsd", 4),
];

#[test]
#[ignore = "opt in: needs the fetched XSDs and xmllint; see the module documentation"]
fn strict_output_meets_its_claims_against_the_release_xsd() {
    let xmllint = xmllint();
    let corpus = corpus();
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ifc-xml-xsd-output");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("create the output directory");

    let mut failures = Vec::new();
    for &(profile, release, file, min_fixtures) in RELEASES {
        let xsd = xsd_path(release, file);
        let label = profile.schema_token();
        let dir = out.join(label);
        std::fs::create_dir_all(&dir).expect("create the release directory");

        let target = target_namespace(&xsd);
        if target != profile.namespace() {
            failures.push(format!(
                "{label}: XmlProfile::namespace is {:?}, the XSD declares {target:?}",
                profile.namespace()
            ));
        }

        let documents = write_corpus(&corpus, profile, &dir, &mut failures);
        if documents.len() < min_fixtures {
            failures.push(format!(
                "{label}: wrote {} documents, expected at least {min_fixtures}",
                documents.len()
            ));
        }

        let well_formed = run(Command::new(&xmllint).arg("--noout").args(&documents));
        if !well_formed.status.success() {
            failures.push(format!(
                "{label}: xmllint finds malformed output:\n{}",
                stderr(&well_formed)
            ));
        }

        let control = dir.join("control.xml");
        std::fs::write(&control, control_document(profile)).expect("write the control");
        check_control_reads(profile, &control, &mut failures);

        let validated = run(Command::new(&xmllint)
            .arg("--noout")
            .arg("--schema")
            .arg(&xsd)
            .arg(&control)
            .args(&documents));
        let uncompilable = UNCOMPILABLE.iter().find(|(known, _)| *known == profile);
        match (validated.status.code(), uncompilable) {
            // xmllint: 5 is "the schema did not compile".
            (Some(5), Some((_, reason))) => {
                println!(
                    "{label}: XSD does not compile, as recorded ({reason}); validation skipped"
                );
            }
            (Some(5), None) => failures.push(format!(
                "{label}: the XSD does not compile:\n{}",
                stderr(&validated)
            )),
            (_, Some(_)) => failures.push(format!(
                "{label}: the XSD now compiles; remove its UNCOMPILABLE entry"
            )),
            (Some(0 | 3), None) => {
                let report = stderr(&validated);
                classify(profile, &control, &documents, &report, &mut failures);
            }
            (code, None) => failures.push(format!(
                "{label}: xmllint exited with {code:?}:\n{}",
                stderr(&validated)
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "strict output departs from its claims:\n{}",
        failures.join("\n")
    );
}

/// The XSD writer's output validates against `IFC4.xsd` with no error.
///
/// Every IFC4 fixture is written with [`XmlCodec::xsd`]; a fixture the
/// writer refuses (a model the configuration cannot carry, see
/// `tests/xsd_corpus_write.rs`) writes no document. Every written document
/// must validate: `xmllint` must report it as valid, with no validity
/// error at all. For IFC4X3 ADD2, whose XSD does not compile
/// ([`UNCOMPILABLE`]), the documents must be well-formed and in the XSD's
/// target namespace.
#[test]
#[ignore = "opt in: needs the fetched XSDs and xmllint; see the module documentation"]
fn xsd_writer_output_validates_against_the_release_xsd() {
    let xmllint = xmllint();
    let corpus = corpus();
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ifc-xml-xsd-writer-output");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("create the output directory");

    let mut failures = Vec::new();
    for &(profile, release, file, min_fixtures) in RELEASES {
        let xsd = xsd_path(release, file);
        let label = profile.schema_token();
        let dir = out.join(label);
        std::fs::create_dir_all(&dir).expect("create the release directory");
        let target = target_namespace(&xsd);
        if target != profile.namespace() {
            failures.push(format!(
                "{label}: XmlProfile::namespace is {:?}, the XSD declares {target:?}",
                profile.namespace()
            ));
        }

        let codec = XmlCodec::xsd(schema(profile), profile);
        let mut documents = Vec::new();
        let mut refused = 0;
        // The hand-built models that exercise every construct, then the
        // corpus.
        let constructs = support::models()
            .into_iter()
            .map(|(name, model)| (format!("construct/{name}"), model));
        let models: Vec<(String, Model)> = constructs.chain(corpus.iter().cloned()).collect();
        for (name, model) in &models {
            if model.header().schema_token() != Some(profile.schema_token()) {
                continue;
            }
            match codec.write_bytes(model) {
                Ok(bytes) => {
                    if let Err(problem) = root_namespace(&bytes, profile) {
                        failures.push(format!("{name}: {problem}"));
                    }
                    let path = dir.join(format!("{}.xml", name.replace('/', "__")));
                    std::fs::write(&path, bytes).expect("write the document");
                    documents.push(path);
                }
                Err(_) => refused += 1,
            }
        }
        if documents.len() < min_fixtures {
            failures.push(format!(
                "{label}: wrote {} documents, expected at least {min_fixtures}",
                documents.len()
            ));
        }

        let well_formed = run(Command::new(&xmllint).arg("--noout").args(&documents));
        if !well_formed.status.success() {
            failures.push(format!(
                "{label}: xmllint finds malformed output:\n{}",
                stderr(&well_formed)
            ));
        }

        let validated = run(Command::new(&xmllint)
            .arg("--noout")
            .arg("--schema")
            .arg(&xsd)
            .args(&documents));
        let report = stderr(&validated);
        let uncompilable = UNCOMPILABLE.iter().find(|(known, _)| *known == profile);
        match (validated.status.code(), uncompilable) {
            (Some(5), Some((_, reason))) => println!(
                "{label}: {} documents written ({refused} refused), well-formed; the XSD does \
                 not compile, as recorded ({reason}), so validation is skipped",
                documents.len()
            ),
            (Some(0), None) => {
                let valid = documents
                    .iter()
                    .filter(|document| {
                        report.contains(&format!("{} validates", document.display()))
                    })
                    .count();
                let errors = report
                    .lines()
                    .filter(|line| line.contains("Schemas validity error"))
                    .count();
                if valid != documents.len() || errors != 0 {
                    failures.push(format!(
                        "{label}: {valid} of {} documents valid, {errors} errors:\n{report}",
                        documents.len()
                    ));
                }
                println!(
                    "{label}: {} documents written ({refused} refused), {valid} valid, {errors} XSD errors",
                    documents.len()
                );
            }
            (_, Some(_)) => failures.push(format!(
                "{label}: the XSD now compiles; remove its UNCOMPILABLE entry"
            )),
            (code, None) => {
                failures.push(format!("{label}: xmllint exited with {code:?}:\n{report}"))
            }
        }
    }
    assert!(
        failures.is_empty(),
        "XSD writer output fails its claims:\n{}",
        failures.join("\n")
    );
}

/// The root of a written document is `ifcXML` in the profile's namespace.
fn root_namespace(bytes: &[u8], profile: XmlProfile) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let root = format!("<ifcXML xmlns=\"{}\"", profile.namespace());
    if text.contains(&root) {
        Ok(())
    } else {
        Err(format!("the root is not `{root}`"))
    }
}

/// Check one release's validation report: the control validates, every
/// written document is refused only for documented reasons.
fn classify(
    profile: XmlProfile,
    control: &Path,
    documents: &[PathBuf],
    report: &str,
    failures: &mut Vec<String>,
) {
    let label = profile.schema_token();
    let control_name = control.display().to_string();
    if !report.contains(&format!("{control_name} validates")) {
        failures.push(format!(
            "{label}: the XSD refuses the control document:\n{}",
            lines_for(report, &control_name).join("\n")
        ));
    }
    let mut seen = vec![0usize; DEVIATIONS.len()];
    for document in documents {
        let name = document.display().to_string();
        let lines = lines_for(report, &name);
        if lines.iter().any(|line| line.ends_with(" validates")) {
            failures.push(format!(
                "{label}: {name} validates; strict output now meets the XSD, \
                 so update this check and the documentation"
            ));
            continue;
        }
        let mut entities = false;
        for line in lines
            .iter()
            .filter(|line| line.contains("Schemas validity error"))
        {
            match DEVIATIONS
                .iter()
                .position(|(_, pattern, _)| matches(profile, pattern, line))
            {
                Some(index) => {
                    seen[index] += 1;
                    entities |= DEVIATIONS[index].0 == "entity element";
                }
                None => failures.push(format!("{label}: undocumented XSD error: {line}")),
            }
        }
        if !entities && has_entities(document) {
            failures.push(format!(
                "{label}: {name} has entities but no entity-element deviation"
            ));
        }
    }
    for ((name, _, reason), count) in DEVIATIONS.iter().zip(&seen) {
        println!("{label}: {count:>3} x {name}: {reason}");
    }
    println!(
        "{label}: {} documents written, all refused by the XSD for documented reasons only",
        documents.len()
    );
}

/// Whether a validity error line matches a deviation pattern.
fn matches(profile: XmlProfile, pattern: &str, line: &str) -> bool {
    let namespace = format!("{{{}}}", profile.namespace());
    pattern.split('|').all(|part| {
        if part.contains("HEADER") {
            HEADER_ELEMENTS
                .iter()
                .any(|element| line.contains(&format!("Element '{namespace}{element}'")))
        } else {
            line.contains(&part.replace("{NS}", &namespace))
        }
    })
}

/// The report lines about one file, which xmllint prefixes with its path.
fn lines_for<'a>(report: &'a str, name: &str) -> Vec<&'a str> {
    report
        .lines()
        .filter(|line| {
            line.strip_prefix(name)
                .is_some_and(|rest| rest.starts_with(':') || rest.starts_with(' '))
        })
        .collect()
}

fn has_entities(document: &Path) -> bool {
    let text = std::fs::read_to_string(document).expect("read written document");
    text.lines().any(|line| line.starts_with("  <IFC"))
}

/// A minimal XSD-configuration document of the release.
fn control_document(profile: XmlProfile) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ifcXML xmlns=\"{}\">\n  \
         <IfcPerson id=\"i1\" Identification=\"P1\" FamilyName=\"Control\"/>\n\
         </ifcXML>\n",
        profile.namespace()
    )
}

/// The control must be a document the XSD reader reads, too.
fn check_control_reads(profile: XmlProfile, control: &Path, failures: &mut Vec<String>) {
    let bytes = std::fs::read(control).expect("read the control");
    let codec = XmlCodec::xsd(schema(profile), profile);
    if let Err(error) = ifc_xml::reader::read(&codec, &bytes) {
        failures.push(format!(
            "{}: XmlCodec::xsd refuses the control document: {error}",
            profile.schema_token()
        ));
    }
}

/// Write every fixture of `profile`'s release with the strict profile.
fn write_corpus(
    corpus: &[(String, Model)],
    profile: XmlProfile,
    dir: &Path,
    failures: &mut Vec<String>,
) -> Vec<PathBuf> {
    let codec = XmlCodec::with_schema_and_profile(schema(profile), profile);
    let mut written = Vec::new();
    for (name, model) in corpus {
        if model.header().schema_token() != Some(profile.schema_token()) {
            continue;
        }
        match codec.write_bytes(model) {
            Ok(bytes) => {
                let path = dir.join(format!("{}.xml", name.replace('/', "__")));
                std::fs::write(&path, bytes).expect("write the document");
                written.push(path);
            }
            Err(error) => failures.push(format!("{name}: strict write failed: {error}")),
        }
    }
    written
}

fn schema(profile: XmlProfile) -> Arc<ifc_schema::Schema> {
    Arc::new(
        ifc_schema::for_version(profile.version())
            .unwrap_or_else(|refused| panic!("{refused}"))
            .clone(),
    )
}

/// Every committed fixture, read with the STEP codec.
fn corpus() -> Vec<(String, Model)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT);
    assert!(
        root.is_dir(),
        "fixture corpus missing at {}",
        root.display()
    );
    let mut found = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read fixture directory") {
            let path = entry.expect("fixture directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ifc"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path).expect("read fixture");
            let model = StepCodec
                .read_bytes(&bytes)
                .unwrap_or_else(|error| panic!("{name}: STEP read failed: {error}"));
            (name, model)
        })
        .collect()
}

/// The fetched XSD; its absence fails the check.
fn xsd_path(release: &str, file: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for base in [
        "../../references/ifc-spec",
        "../../../../references/ifc-spec",
    ] {
        let path = root.join(base).join(release).join(file);
        if path.is_file() {
            return path;
        }
    }
    panic!("references/ifc-spec/{release}/{file} not found; run scripts/fetch-ifc-schemas.sh");
}

/// The `targetNamespace` the XSD's root declares.
fn target_namespace(xsd: &Path) -> String {
    let text = std::fs::read_to_string(xsd).expect("read the XSD");
    let key = "targetNamespace=\"";
    let start = text.find(key).expect("the XSD declares a targetNamespace") + key.len();
    let end = text[start..].find('"').expect("closing quote") + start;
    text[start..end].to_string()
}

/// The validator; its absence fails the check.
fn xmllint() -> PathBuf {
    let binary =
        std::env::var_os("XMLLINT").map_or_else(|| PathBuf::from("xmllint"), PathBuf::from);
    let probe = Command::new(&binary).arg("--version").output();
    assert!(
        probe.is_ok_and(|output| output.status.success()),
        "{} not runnable; install libxml2-utils or set XMLLINT",
        binary.display()
    );
    binary
}

fn run(command: &mut Command) -> Output {
    command.output().expect("run xmllint")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
