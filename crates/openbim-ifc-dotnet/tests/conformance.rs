//! The C# binding against the two contracts it depends on, without a .NET
//! SDK: the C header, and the shared binding core's domain records.
//!
//! P/Invoke declarations and positional records are written by hand. A
//! header change the C# did not follow, or a field the core added, would
//! otherwise surface only as a crash or a misread field at run time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .replace("\r\n", "\n")
}

fn header() -> String {
    read(&root().join("../openbim-ifc-capi/include/openbim_ifc.h"))
}

fn csharp(relative: &str) -> String {
    read(&root().join("dotnet/OpenBim.Ifc").join(relative))
}

/// `SCREAMING_SNAKE` or `snake_case` to `PascalCase`.
fn pascal(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            let mut chars = lower.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// Every `<prefix>name(params);` declaration: name and parameter list.
fn declarations(text: &str, prefix: &str) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let mut rest = text;
    while let Some(at) = rest.find(prefix) {
        let after = &rest[at + prefix.len()..];
        let open = after.find('(').expect("a declaration has a parameter list");
        let name = after[..open].trim().to_owned();
        let close = after.find(");").expect("a declaration ends with `);`");
        let params = after[open + 1..close]
            .split(',')
            .map(|param| param.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|param| !param.is_empty())
            .collect();
        out.insert(name, params);
        rest = &after[close..];
    }
    out
}

/// The C# spelling of a C parameter type, as `NativeMethods.cs` declares it.
fn csharp_type(c_param: &str) -> String {
    let pointer = c_param.contains('*');
    let base = c_param
        .replace("const ", "")
        .replace('*', " ")
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned();
    let mapped = match base.as_str() {
        "OpenbimIfcModel" | "uint64_t" => "ulong",
        "size_t" => "nuint",
        "uint32_t" => "uint",
        "uint8_t" => "byte",
        "OpenbimIfcValueNode" => "ValueNode",
        "OpenbimIfcValidationSummary" => "NativeValidationSummary",
        "OpenbimIfcVersion" => "NativeVersion",
        other => panic!("no C# type for the C type `{other}`; extend this table"),
    };
    format!("{mapped}{}", if pointer { "*" } else { "" })
}

#[test]
fn every_c_function_is_declared_with_the_headers_parameters() {
    let c = declarations(&header(), "OpenbimIfcStatus openbim_ifc_v0_1_");
    let cs = declarations(
        &csharp("Native/NativeMethods.cs"),
        "internal static extern IfcStatus openbim_ifc_v0_1_",
    );
    // Guards the parse: the header has dozens of exports.
    assert!(c.len() >= 40, "found only {} C functions", c.len());
    let missing: Vec<_> = c.keys().filter(|name| !cs.contains_key(*name)).collect();
    let extra: Vec<_> = cs.keys().filter(|name| !c.contains_key(*name)).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "NativeMethods.cs lacks {missing:?} and declares {extra:?} the header does not"
    );
    for (name, params) in &c {
        let expected: Vec<String> = params.iter().map(|p| csharp_type(p)).collect();
        let declared: Vec<String> = cs[name]
            .iter()
            .map(|p| {
                p.rsplit_once(' ')
                    .map_or(p.as_str(), |(ty, _)| ty)
                    .to_owned()
            })
            .collect();
        assert_eq!(
            declared, expected,
            "openbim_ifc_v0_1_{name}: C# parameter types differ from the header's {params:?}"
        );
    }
}

/// `PREFIXNAME = value` pairs (C enum) or `#define PREFIXNAME value`.
fn c_constants(text: &str, prefix: &str) -> BTreeMap<String, i64> {
    text.lines()
        .filter_map(|line| {
            let line = line
                .trim()
                .trim_start_matches("#define ")
                .trim_end_matches(',');
            let rest = line.strip_prefix(prefix)?;
            let (name, value) = rest.split_once(" = ").or_else(|| rest.split_once(' '))?;
            Some((pascal(name.trim()), value.trim().parse().ok()?))
        })
        .collect()
}

/// `Name = value,` members of a C# enum, or `public const int Name = value;`.
fn csharp_constants(text: &str, start: &str) -> BTreeMap<String, i64> {
    let body = &text[text.find(start).unwrap_or_else(|| panic!("no `{start}`"))..];
    let body = &body[..body.find("\n}").expect("a closing brace")];
    body.lines()
        .filter_map(|line| {
            let line = line
                .trim()
                .trim_start_matches("public const int ")
                .trim_start_matches("public const uint ")
                .trim_end_matches([',', ';']);
            let (name, value) = line.split_once(" = ")?;
            Some((name.trim().to_owned(), value.trim().parse().ok()?))
        })
        .collect()
}

#[test]
fn every_status_kind_and_flag_has_the_headers_value() {
    let header = header();
    let statuses = c_constants(&header, "OPENBIM_IFC_STATUS_");
    assert!(
        statuses.len() >= 20,
        "found only {} statuses",
        statuses.len()
    );
    assert_eq!(
        csharp_constants(&csharp("IfcStatus.cs"), "public enum IfcStatus"),
        statuses,
        "IfcStatus.cs differs from OpenbimIfcStatus"
    );

    let kinds = c_constants(&header, "OPENBIM_IFC_KIND_");
    assert_eq!(kinds.len(), 12);
    let native = csharp("Native/NativeMethods.cs");
    assert_eq!(
        csharp_constants(&native, "internal static class Kind"),
        kinds
    );

    let mut flags = c_constants(&header, "OPENBIM_IFC_PARSE_");
    // A preset, not a flag: ParseOptions.Lenient is the C# form.
    flags.remove("Lenient");
    assert_eq!(flags.len(), 3);
    assert_eq!(
        csharp_constants(&native, "internal static class ParseFlags"),
        flags
    );
}

/// The field names of `typedef struct { ... } Name;` in the header.
fn c_struct(header: &str, name: &str) -> Vec<String> {
    let end = header
        .find(&format!("}} {name};"))
        .unwrap_or_else(|| panic!("no struct {name}"));
    let start = header[..end]
        .rfind("typedef struct {")
        .expect("struct start");
    header[start..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('/') || line.starts_with('*') || !line.ends_with(';') {
                return None;
            }
            Some(pascal(
                line.trim_end_matches(';').split_whitespace().last()?,
            ))
        })
        .collect()
}

/// The public field names of `struct Name` in C#.
fn csharp_struct(text: &str, name: &str) -> Vec<String> {
    let body = &text[text.find(&format!("struct {name}")).expect("a C# struct")..];
    let body = &body[..body.find('}').expect("struct end")];
    body.lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("public ")?;
            Some(
                line.trim_end_matches(';')
                    .split_whitespace()
                    .last()?
                    .to_owned(),
            )
        })
        .collect()
}

#[test]
fn every_struct_has_the_headers_fields_in_order() {
    let header = header();
    let native = csharp("Native/NativeMethods.cs");
    for (c, cs) in [
        ("OpenbimIfcValueNode", "ValueNode"),
        ("OpenbimIfcValidationSummary", "NativeValidationSummary"),
        ("OpenbimIfcVersion", "NativeVersion"),
    ] {
        let fields = c_struct(&header, c);
        assert!(fields.len() >= 6, "{c}: parsed {fields:?}");
        assert_eq!(csharp_struct(&native, cs), fields, "{cs} differs from {c}");
    }
}

/// Each `Record::new("Name", vec![("field", ...), ...])` of the binding
/// core: the record's name and its field names, in order. A field is a
/// string literal opening a tuple directly inside the `vec![...]`.
fn core_records() -> BTreeMap<String, Vec<String>> {
    let dir = root().join("../openbim-ifc-binding-core/src");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect("binding core sources") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let text = read(&path);
        let mut search = 0;
        while let Some(at) = text[search..].find("Record::new(") {
            let start = search + at + "Record::new(".len();
            search = start;
            let (name, fields) = parse_record(&text[start..]);
            out.insert(name, fields);
        }
    }
    out
}

/// Parse from just after `Record::new(` to its closing parenthesis.
fn parse_record(text: &str) -> (String, Vec<String>) {
    let mut depth = 0i32; // nesting of (), [] relative to `Record::new(`
    let mut name = None;
    let mut fields = Vec::new();
    let mut previous = '(';
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            '"' => {
                let end = text[index + 1..].find('"').expect("closing quote") + index + 1;
                let literal = text[index + 1..end].to_owned();
                if depth == 0 && name.is_none() {
                    name = Some(literal);
                } else if depth == 2 && previous == '(' {
                    // `vec![` is depth 1, a field tuple's `(` depth 2.
                    fields.push(literal);
                }
                while chars.peek().is_some_and(|(i, _)| *i <= end) {
                    chars.next();
                }
                previous = '"';
                continue;
            }
            _ => {}
        }
        if !c.is_whitespace() {
            previous = c;
        }
    }
    (name.expect("a record name"), fields)
}

/// Each `public sealed record Name(` in the C# domain sources: the name
/// and its positional parameter names.
fn csharp_records() -> BTreeMap<String, Vec<String>> {
    let dir = root().join("dotnet/OpenBim.Ifc/Domains");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect("C# domain sources") {
        let text = read(&entry.expect("entry").path());
        let mut rest = text.as_str();
        while let Some(at) = rest.find("public sealed record ") {
            let after = &rest[at + "public sealed record ".len()..];
            let open = after.find('(').expect("a positional record");
            let name = after[..open].trim().to_owned();
            let close = after.find(')').expect("parameter list end");
            let params = after[open + 1..close]
                .split(',')
                .filter_map(|param| {
                    let param = param.split('=').next()?.trim();
                    Some(param.split_whitespace().last()?.to_owned())
                })
                .collect();
            out.insert(name, params);
            rest = &after[close..];
        }
    }
    out
}

/// C# names that differ from the core's: `System` would hide the `System`
/// namespace, and a record cannot have a member named like itself.
const RENAMED: &[(&str, &str)] = &[("IfcSystem", "System"), ("SystemsView", "Systems")];

/// Core records with no C# record: the C ABI returns no `PropertyEditResult`
/// (`set_properties` writes one id per edit instead); `PropertyEdit` is
/// C# input, not a tape record; `Example` is the core's own unit test.
const NOT_DECODED: &[&str] = &["PropertyEditResult", "Example"];

#[test]
fn every_domain_record_has_the_cores_fields_in_order() {
    let core = core_records();
    assert!(core.len() >= 30, "found only {} core records", core.len());
    let cs = csharp_records();
    let mut problems = Vec::new();
    for (name, fields) in &cs {
        if name == "PropertyEdit" {
            continue;
        }
        let core_name = RENAMED
            .iter()
            .find(|(csharp, _)| csharp == name)
            .map_or(name.as_str(), |(_, core)| core);
        match core.get(core_name) {
            None => problems.push(format!("{name}: no core record `{core_name}`")),
            Some(expected) => {
                let expected: Vec<String> = expected.iter().map(|f| pascal(f)).collect();
                if &expected != fields {
                    problems.push(format!("{name}: C# {fields:?}, core {expected:?}"));
                }
            }
        }
    }
    for name in core.keys() {
        let csharp = RENAMED
            .iter()
            .find(|(_, core)| core == name)
            .map_or(name.as_str(), |(csharp, _)| csharp);
        if !cs.contains_key(csharp) && !NOT_DECODED.contains(&name.as_str()) {
            problems.push(format!("core record `{name}` has no C# record"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_parsers_find_what_they_must() {
    let (name, fields) = parse_record(
        "\"Tree\", vec![(\"a\", Field::Id(1)), (\n \"b_c\",\n Field::List(x.map(|_| Field::Record(Record::new(\"In\", vec![(\"z\", Field::Null)])))),\n ),],)",
    );
    assert_eq!(name, "Tree");
    assert_eq!(fields, ["a", "b_c"]);
    assert_eq!(pascal("metres_per_unit"), "MetresPerUnit");
    assert_eq!(pascal("BUFFER_TOO_SMALL"), "BufferTooSmall");
    assert_eq!(csharp_type("const uint8_t *type_name"), "byte*");
    assert_eq!(csharp_type("OpenbimIfcModel model"), "ulong");
}
