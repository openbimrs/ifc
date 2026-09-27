//! The `openbim_ifc` Python package, read with Python's own `ast` module.
//!
//! The public API is plain Python over a private native module, so its
//! signatures and docstrings are the source. Reading them with `ast` needs no
//! build and no import: the package is parsed, never executed.

use std::process::Command;

use super::summary;
use crate::workspace::Workspace;

const MODEL: &str = "crates/openbim-ifc-py/python/openbim_ifc/model.py";
const VALUES: &str = "crates/openbim-ifc-py/python/openbim_ifc/values.py";

/// Prints `{name, kind, params, returns, doc}` per public member of
/// `IfcModel`, and `{name, fields, doc}` per value dataclass.
const SCRIPT: &str = r#"
import ast, json, sys

def ann(node):
    # Forward references are strings; show them as the type they name.
    return ast.unparse(node).replace("'", "").replace('"', "")

def params(fn):
    a = fn.args
    positional = a.posonlyargs + a.args
    defaults = [None] * (len(positional) - len(a.defaults)) + list(a.defaults)
    out = []
    for arg, default in list(zip(positional, defaults))[1:]:
        text = arg.arg + (": " + ann(arg.annotation) if arg.annotation else "")
        out.append(text + (" = " + ast.unparse(default) if default is not None else ""))
    if a.kwonlyargs:
        out.append("*")
        for arg, default in zip(a.kwonlyargs, a.kw_defaults):
            text = arg.arg + (": " + ann(arg.annotation) if arg.annotation else "")
            out.append(text + (" = " + ast.unparse(default) if default is not None else ""))
    return out

model = ast.parse(open(sys.argv[1]).read())
members = []
for node in model.body:
    if isinstance(node, ast.ClassDef) and node.name == "IfcModel":
        for fn in node.body:
            if not isinstance(fn, ast.FunctionDef):
                continue
            decorators = [ast.unparse(d) for d in fn.decorator_list]
            kind = "classmethod" if "classmethod" in decorators else \
                   "property" if "property" in decorators else "method"
            members.append({
                "name": fn.name, "kind": kind, "params": params(fn),
                "returns": ann(fn.returns) if fn.returns else None,
                "doc": ast.get_docstring(fn) or "",
            })

values = []
for node in ast.parse(open(sys.argv[2]).read()).body:
    if isinstance(node, ast.ClassDef) and any("dataclass" in ast.unparse(d) for d in node.decorator_list):
        fields = [ast.unparse(s.target) + ": " + ann(s.annotation)
                  for s in node.body if isinstance(s, ast.AnnAssign)]
        values.append({"name": node.name, "fields": fields, "doc": ast.get_docstring(node) or ""})

print(json.dumps({"members": members, "values": values}))
"#;

pub(super) fn reference(workspace: &Workspace) -> Result<String, String> {
    let output = Command::new("python3")
        .arg("-c")
        .arg(SCRIPT)
        .arg(workspace.root.join(MODEL))
        .arg(workspace.root.join(VALUES))
        .output()
        .map_err(|error| format!("python3 is needed to read the Python API: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "reading {MODEL}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("reading {MODEL}: {error}"))?;
    let text = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    };

    let mut rows = vec![
        "| Member | Description |".to_owned(),
        "| --- | --- |".to_owned(),
    ];
    for member in json["members"].as_array().into_iter().flatten() {
        let name = text(member, "name");
        let params: Vec<&str> = member["params"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p.as_str())
            .collect();
        let returns = member["returns"]
            .as_str()
            .map(|r| format!(" -> {r}"))
            .unwrap_or_default();
        let signature = match (name.as_str(), text(member, "kind").as_str()) {
            ("__init__", _) => format!("IfcModel({})", params.join(", ")),
            ("__len__", _) => format!("len(model){returns}"),
            (dunder, _) if dunder.starts_with('_') => continue,
            (_, "classmethod") => format!("IfcModel.{name}({}){returns}", params.join(", ")),
            (_, "property") => format!("model.{name}{}", returns.replacen(" ->", ":", 1)),
            _ => format!("model.{name}({}){returns}", params.join(", ")),
        };
        let doc = match name.as_str() {
            "__len__" => "Number of entities.".to_owned(),
            _ => summary(&text(member, "doc")),
        };
        rows.push(format!("| `{}` | {doc} |", signature.replace('|', "\\|")));
    }

    let mut values = vec![
        String::new(),
        "Attribute values are frozen dataclasses in `openbim_ifc` (from `openbim_ifc.values`):"
            .to_owned(),
        String::new(),
        "| Value | Meaning |".to_owned(),
        "| --- | --- |".to_owned(),
    ];
    for value in json["values"].as_array().into_iter().flatten() {
        let fields: Vec<&str> = value["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f.as_str())
            .collect();
        values.push(format!(
            "| `{}({})` | {} |",
            text(value, "name"),
            fields.join(", "),
            summary(&text(value, "doc")).replace("``", "`")
        ));
    }
    Ok(format!(
        "{}\n{}",
        rows.join("\n").replace("``", "`"),
        values.join("\n")
    ))
}
