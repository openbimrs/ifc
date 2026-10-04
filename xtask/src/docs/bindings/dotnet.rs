//! The `OpenBim.Ifc` .NET package: `IfcModel`'s public members and the
//! `Value` cases, read from the C# source and its XML doc comments.
//!
//! The package is hand-written C#, so its declarations and `<summary>` docs
//! are the source, as the Python module is for the Python page. Each public
//! member's declaration sits on one line; the C# compiler checks the docs
//! (`GenerateDocumentationFile` with warnings as errors), so every member
//! read here has a summary.

use super::summary;
use crate::workspace::Workspace;

const MODEL: &str = "crates/openbim-ifc-dotnet/dotnet/OpenBim.Ifc/IfcModel.cs";
const VALUES: &str = "crates/openbim-ifc-dotnet/dotnet/OpenBim.Ifc/Value.cs";

pub(super) fn reference(workspace: &Workspace) -> Result<String, String> {
    let read = |path: &str| {
        std::fs::read_to_string(workspace.root.join(path))
            .map(|text| text.replace("\r\n", "\n"))
            .map_err(|error| format!("{path}: {error}"))
    };
    let model = read(MODEL)?;
    let model_members = members(&model);
    if model_members.len() < 20 {
        return Err(format!(
            "{MODEL}: found only {} public members",
            model_members.len()
        ));
    }
    let mut rows = vec![
        "| Member | Description |".to_owned(),
        "| --- | --- |".to_owned(),
    ];
    for (signature, doc) in &model_members {
        let shown = match signature.strip_prefix("IfcModel(") {
            Some(rest) => format!("new IfcModel({rest}"),
            None => signature.clone(),
        };
        rows.push(format!(
            "| `{}` | {} |",
            shown.replace('|', "\\|"),
            summary(doc)
        ));
    }

    let values = members(&read(VALUES)?)
        .into_iter()
        .filter(|(signature, _)| signature.starts_with("sealed record "))
        .map(|(signature, doc)| {
            let case = signature
                .trim_start_matches("sealed record ")
                .trim_end_matches(" : Value");
            format!("| `Value.{case}` | {} |", summary(&doc))
        })
        .collect::<Vec<_>>();
    if values.len() != 12 {
        return Err(format!(
            "{VALUES}: expected 12 value cases, found {}",
            values.len()
        ));
    }
    Ok(format!(
        "{}\n\nAttribute values are the cases of the closed record `Value`:\n\n| Value | Meaning |\n| --- | --- |\n{}",
        rows.join("\n"),
        values.join("\n")
    ))
}

/// Each `public` declaration after the first (the type itself), as its
/// signature without `public` and with a property's accessors, and the text
/// of its `<summary>`.
fn members(source: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut doc = Vec::new();
    let mut seen_type = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(text) = trimmed.strip_prefix("///") {
            doc.push(text.trim());
            continue;
        }
        let Some(declaration) = trimmed.strip_prefix("public ") else {
            if !trimmed.starts_with('[') {
                doc.clear();
            }
            continue;
        };
        let text = summary_text(&doc.join(" "));
        doc.clear();
        if !seen_type {
            seen_type = true;
            continue;
        }
        if declaration.starts_with("override ") {
            continue;
        }
        let head = declaration
            .split(" => ")
            .next()
            .unwrap_or(declaration)
            .trim_end_matches(['{', ';', ' '])
            .to_owned();
        let signature = if head.contains('(') || head.contains("record ") {
            head
        } else if declaration.contains(" => ") {
            format!("{head} {{ get; }}")
        } else {
            format!("{head} {}", accessors(&lines[index + 1..]))
        };
        out.push((signature, text));
    }
    out
}

/// `{ get; }` or `{ get; set; }` from a property's body.
fn accessors(body: &[&str]) -> &'static str {
    let mut depth = 0;
    for line in body {
        let trimmed = line.trim();
        if trimmed == "set" || trimmed.starts_with("set ") || trimmed.starts_with("set;") {
            return "{ get; set; }";
        }
        depth += line.matches('{').count();
        depth -= line.matches('}').count().min(depth);
        if depth == 0 && trimmed.ends_with('}') {
            break;
        }
    }
    "{ get; }"
}

/// The `<summary>` of an XML doc comment as Markdown: `<c>x</c>` and
/// `<see cref="X"/>` become code, tags are dropped.
fn summary_text(xml: &str) -> String {
    let body = xml
        .split("<summary>")
        .nth(1)
        .and_then(|rest| rest.split("</summary>").next())
        .unwrap_or("");
    let mut out = String::new();
    let mut rest = body;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let close = rest[open..]
            .find('>')
            .map_or(rest.len(), |at| open + at + 1);
        let tag = &rest[open..close];
        if tag == "<c>" || tag == "</c>" {
            out.push('`');
        } else if let Some(name) = attribute(tag, "cref").or_else(|| attribute(tag, "name")) {
            out.push_str(&format!("`{name}`"));
        }
        rest = &rest[close..];
    }
    out.push_str(rest);
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let end = start + tag[start..].find('"')?;
    Some(&tag[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_carry_their_signature_and_summary() {
        let source = "/// <summary>The type.</summary>\npublic sealed class IfcModel\n{\n    \
            /// <summary>Parse <paramref name=\"data\"/> as <c>STEP</c>.</summary>\n    \
            public static IfcModel Parse(byte[] data)\n    {\n    }\n\n    \
            /// <summary>The header.</summary>\n    public Header Header\n    {\n        get\n        {\n        }\n        set\n        {\n        }\n    }\n\n    \
            /// <summary>Disposed.</summary>\n    public bool IsDisposed => handle.IsClosed;\n\n    \
            /// <inheritdoc/>\n    public override string ToString() => \"\";\n}\n";
        let members = members(source);
        assert_eq!(
            members,
            [
                (
                    "static IfcModel Parse(byte[] data)".to_owned(),
                    "Parse `data` as `STEP`.".to_owned()
                ),
                (
                    "Header Header { get; set; }".to_owned(),
                    "The header.".to_owned()
                ),
                (
                    "bool IsDisposed { get; }".to_owned(),
                    "Disposed.".to_owned()
                ),
            ]
        );
    }
}
