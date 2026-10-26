//! The JavaScript/TypeScript surface of `openbim-ifc-wasm`.
//!
//! Read from the `#[wasm_bindgen]` exports themselves: JS names, getters and
//! constructors come from the attribute, parameter and return types from the
//! Rust signature (or its `unchecked_*_type` override), descriptions from the
//! doc comments. A Rust type with no known TypeScript spelling is an error, so
//! the page can never show a guessed type.

use quote::ToTokens;
use syn::{Attribute, Expr, ExprLit, FnArg, ImplItem, Item, Lit, Meta, Pat, ReturnType, Type};

use super::summary;
use crate::workspace::Workspace;

const MODEL: &str = "crates/openbim-ifc-wasm/src/model.rs";
const TYPES: &str = "crates/openbim-ifc-wasm/src/model/types.rs";
/// Submodules of `model.rs` that add `#[wasm_bindgen] impl IfcModel`
/// blocks, listed after it in this order.
/// [`reference()`] refuses a file below `model/` that has such a block and is
/// not listed, so a new part cannot silently drop its methods from the page.
const MODEL_PARTS: &[&str] = &[
    "crates/openbim-ifc-wasm/src/model/copying.rs",
    "crates/openbim-ifc-wasm/src/model/inverse.rs",
    "crates/openbim-ifc-wasm/src/model/removal.rs",
    "crates/openbim-ifc-wasm/src/model/selector.rs",
    "crates/openbim-ifc-wasm/src/model/table.rs",
    "crates/openbim-ifc-wasm/src/model/units.rs",
    "crates/openbim-ifc-wasm/src/model/validation.rs",
];

/// Every `.rs` file below `model/` with a `#[wasm_bindgen] impl IfcModel`
/// block must be in [`MODEL_PARTS`].
fn check_parts_listed(workspace: &Workspace) -> Result<(), String> {
    let dir = workspace.root.join("crates/openbim-ifc-wasm/src/model");
    let entries = std::fs::read_dir(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let rel = format!("crates/openbim-ifc-wasm/src/model/{name}");
        if MODEL_PARTS.contains(&rel.as_str()) {
            continue;
        }
        let file = parse(workspace, &rel)?;
        let has_block = file.items.iter().any(|item| {
            matches!(item, Item::Impl(block) if block.attrs.iter().any(|a| a.path().is_ident("wasm_bindgen")))
        });
        if has_block {
            return Err(format!(
                "{rel} has a `#[wasm_bindgen] impl` block but is not in MODEL_PARTS \
                 (xtask/src/docs/bindings/js.rs); its methods would be missing from the page"
            ));
        }
    }
    Ok(())
}

pub(super) fn reference(workspace: &Workspace) -> Result<String, String> {
    check_parts_listed(workspace)?;
    let mut files = vec![parse(workspace, MODEL)?];
    for part in MODEL_PARTS {
        files.push(parse(workspace, part)?);
    }
    let mut rows = vec![
        "| Member | Throws `IfcError` | Description |".to_owned(),
        "| --- | --- | --- |".to_owned(),
    ];
    for item in files.iter().flat_map(|file| &file.items) {
        let Item::Impl(block) = item else { continue };
        if !block
            .attrs
            .iter()
            .any(|a| a.path().is_ident("wasm_bindgen"))
        {
            continue;
        }
        for member in &block.items {
            let ImplItem::Fn(function) = member else {
                continue;
            };
            let export = options(&function.attrs);
            let name = option(&export, "js_name").unwrap_or_else(|| function.sig.ident.to_string());
            let receiver = function
                .sig
                .inputs
                .iter()
                .any(|a| matches!(a, FnArg::Receiver(_)));
            let mut params = Vec::new();
            for input in &function.sig.inputs {
                let FnArg::Typed(typed) = input else { continue };
                let param = options(&typed.attrs);
                let ident = match &*typed.pat {
                    Pat::Ident(ident) => ident.ident.to_string(),
                    _ => "value".to_owned(),
                };
                let js = option(&param, "js_name").unwrap_or(ident);
                let ty = match option(&param, "unchecked_param_type") {
                    Some(ty) => ty,
                    None => typescript(&typed.ty)?,
                };
                params.push(format!("{js}: {ty}"));
            }
            let (returns, throws) = match &function.sig.output {
                ReturnType::Default => ("void".to_owned(), false),
                ReturnType::Type(_, ty) => {
                    let (inner, throws) = unwrap_result(ty);
                    let ty = match option(&export, "unchecked_return_type") {
                        Some(ty) => ty,
                        None => typescript(inner)?,
                    };
                    (ty, throws)
                }
            };
            let signature = if flag(&export, "constructor") {
                format!("new IfcModel({})", params.join(", "))
            } else if flag(&export, "getter") {
                format!("model.{name}: {returns}")
            } else if receiver {
                format!("model.{name}({}): {returns}", params.join(", "))
            } else {
                format!("IfcModel.{name}({}): {returns}", params.join(", "))
            };
            rows.push(format!(
                "| `{}` | {} | {} |",
                signature.replace('|', "\\|"),
                if throws { "yes" } else { "" },
                summary(&docs(&function.attrs))
            ));
        }
    }
    let types = custom_types(&parse(workspace, TYPES)?)?;
    Ok(format!(
        "{}\n\nAttribute values and error codes are typed by the package's `.d.ts`:\n\n```ts\n{}\n```",
        rows.join("\n"),
        types.trim()
    ))
}

fn parse(workspace: &Workspace, rel: &str) -> Result<syn::File, String> {
    let source = std::fs::read_to_string(workspace.root.join(rel))
        .map_err(|error| format!("{rel}: {error}"))?;
    syn::parse_file(&source).map_err(|error| format!("{rel}: {error}"))
}

/// The `typescript_custom_section` constants, concatenated.
fn custom_types(file: &syn::File) -> Result<String, String> {
    let mut out = Vec::new();
    for item in &file.items {
        let Item::Const(constant) = item else {
            continue;
        };
        if !flag(&options(&constant.attrs), "typescript_custom_section") {
            continue;
        }
        let Expr::Lit(ExprLit {
            lit: Lit::Str(text),
            ..
        }) = &*constant.expr
        else {
            return Err(format!(
                "{TYPES}: a typescript_custom_section must be a string literal"
            ));
        };
        out.push(text.value());
    }
    Ok(out.join("\n"))
}

/// `wasm_bindgen(...)` arguments, split at top-level commas.
fn options(attrs: &[Attribute]) -> Vec<String> {
    let mut out = Vec::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("wasm_bindgen")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        let tokens = list.tokens.to_string();
        let (mut current, mut quoted) = (String::new(), false);
        for c in tokens.chars() {
            match c {
                '"' => {
                    quoted = !quoted;
                    current.push(c);
                }
                ',' if !quoted => out.push(std::mem::take(&mut current).trim().to_owned()),
                _ => current.push(c),
            }
        }
        if !current.trim().is_empty() {
            out.push(current.trim().to_owned());
        }
    }
    out
}

fn flag(options: &[String], name: &str) -> bool {
    options.iter().any(|o| o == name)
}

fn option(options: &[String], name: &str) -> Option<String> {
    options.iter().find_map(|o| {
        let (key, value) = o.split_once('=')?;
        (key.trim() == name).then(|| value.trim().trim_matches('"').to_owned())
    })
}

fn docs(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(meta) if meta.path.is_ident("doc") => match &meta.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            },
            _ => None,
        })
        .map(|line| line.strip_prefix(' ').unwrap_or(&line).to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `Result<T, JsValue>` → (`T`, throws).
fn unwrap_result(ty: &Type) -> (&Type, bool) {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return (inner, true);
                    }
                }
            }
        }
    }
    (ty, false)
}

/// The TypeScript spelling of a Rust type crossing the wasm boundary.
fn typescript(ty: &Type) -> Result<String, String> {
    let rust: String = ty
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    Ok(match rust.as_str() {
        "&[u8]" | "Uint8Array" => "Uint8Array",
        "&str" | "String" => "string",
        "u64" | "i64" => "bigint",
        "u32" | "i32" | "usize" | "f64" => "number",
        "bool" => "boolean",
        "()" => "void",
        "IfcModel" | "&IfcModel" => "IfcModel",
        "Vec<String>" => "string[]",
        "Option<String>" => "string | undefined",
        "Option<u32>" => "number | undefined",
        _ => {
            return Err(format!(
                "{MODEL}: no TypeScript spelling for `{rust}`; add `unchecked_param_type` / \
                 `unchecked_return_type` on the export or a mapping in xtask"
            ))
        }
    }
    .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_options_split_outside_quotes() {
        let attr: Attribute = syn::parse_quote!(#[wasm_bindgen(js_name = x, unchecked_return_type = "[bigint, bigint][]")]);
        let options = options(&[attr]);
        assert_eq!(option(&options, "js_name").as_deref(), Some("x"));
        assert_eq!(
            option(&options, "unchecked_return_type").as_deref(),
            Some("[bigint, bigint][]")
        );
    }

    #[test]
    fn unknown_rust_types_are_refused() {
        let ty: Type = syn::parse_quote!(HashMap<u8, u8>);
        assert!(typescript(&ty).is_err());
        let ty: Type = syn::parse_quote!(Option<String>);
        assert_eq!(typescript(&ty).unwrap(), "string | undefined");
    }
}
