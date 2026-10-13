//! The ZIP container: finding the model entry, inflating it within the
//! budget, and packing a document into a new archive.

use std::io::{Cursor, Read, Write};

use zip::{CompressionMethod, ZipArchive};

use super::{Budget, IfcZipError, InnerFormat};

/// The model document an ifcZIP archive holds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry {
    /// The entry's name in the archive, e.g. `model.ifc`.
    pub name: String,
    /// The document's format, from its content.
    pub format: InnerFormat,
    /// The inflated document.
    pub bytes: Vec<u8>,
}

/// Is `name` a model entry: a file at the archive root ending in `.ifc` or
/// `.ifcXML`, any case?
pub(super) fn is_model_name(name: &str) -> bool {
    !name.contains(['/', '\\']) && model_extension(name).is_some()
}

/// The format a model entry's extension names.
fn model_extension(name: &str) -> Option<InnerFormat> {
    let (_, extension) = name.rsplit_once('.')?;
    if extension.eq_ignore_ascii_case("ifc") {
        Some(InnerFormat::Step)
    } else if extension.eq_ignore_ascii_case("ifcxml") {
        Some(InnerFormat::IfcXml)
    } else {
        None
    }
}

/// The model document in an ifcZIP archive, inflated within `budget`.
///
/// The model entry is the one `.ifc` or `.ifcXML` file at the archive root
/// (buildingSMART implementation agreement CV-2x3-154: "There shall be only
/// a single *.ifc file (or only a single ifcXML file) in the main
/// directory"); entries in subdirectories are left alone, since the same
/// agreement reserves them for linked documents. Its format is read from
/// its content, not its name.
///
/// # Errors
///
/// [`IfcZipError::Malformed`] for bytes that do not read as a ZIP archive
/// or an entry whose checksum does not match; [`IfcZipError::NoModel`],
/// [`IfcZipError::SeveralModels`], [`IfcZipError::Encrypted`],
/// [`IfcZipError::UnsupportedCompression`] (the agreement's PKZIP 2.04g
/// allows Stored and Deflate only), [`IfcZipError::BudgetExceeded`] and
/// [`IfcZipError::UnknownInnerFormat`] as named.
pub fn extract(bytes: &[u8], budget: &Budget) -> Result<Entry, IfcZipError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| IfcZipError::Malformed(e.to_string()))?;
    let mut files = Vec::new();
    let mut models = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|e| IfcZipError::Malformed(e.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .map_or_else(|_| "<non-UTF-8 name>".to_owned(), |name| name.into_owned());
        if is_model_name(&name) {
            models.push((index, name.clone()));
        }
        files.push(name);
    }
    let (index, name) = match models.len() {
        1 => models.remove(0),
        0 => return Err(IfcZipError::NoModel { entries: files }),
        _ => {
            return Err(IfcZipError::SeveralModels {
                names: models.into_iter().map(|(_, name)| name).collect(),
            })
        }
    };

    let (encrypted, method, compressed, declared) = {
        let raw = archive
            .by_index_raw(index)
            .map_err(|e| IfcZipError::Malformed(e.to_string()))?;
        (
            raw.encrypted(),
            raw.compression(),
            raw.compressed_size(),
            raw.size(),
        )
    };
    if encrypted {
        return Err(IfcZipError::Encrypted { name });
    }
    if !matches!(
        method,
        CompressionMethod::Stored | CompressionMethod::Deflated
    ) {
        #[allow(deprecated)]
        let number = method.to_u16();
        return Err(IfcZipError::UnsupportedCompression {
            name,
            method: format!("{} ({number})", CompressionMethod::name_from_u16(number)),
        });
    }
    // The compressed size is bounded by the archive itself, whatever the
    // header claims.
    let compressed = compressed.min(bytes.len() as u64);
    let (allowed, limit) = budget.allowance(compressed);
    if declared > allowed {
        return Err(IfcZipError::BudgetExceeded { name, limit });
    }

    let reader = archive
        .by_index(index)
        .map_err(|e| IfcZipError::Malformed(e.to_string()))?;
    // Reserve what the header declares, which is already within budget; the
    // read below stops one byte past the allowance whatever it declared.
    let mut out = Vec::with_capacity(usize::try_from(declared).unwrap_or(0));
    reader
        .take(allowed.saturating_add(1))
        .read_to_end(&mut out)
        .map_err(|e| IfcZipError::Malformed(format!("entry {name:?}: {e}")))?;
    if out.len() as u64 > allowed {
        return Err(IfcZipError::BudgetExceeded { name, limit });
    }
    let format = InnerFormat::of_content(&out)
        .ok_or(IfcZipError::UnknownInnerFormat { name: name.clone() })?;
    Ok(Entry {
        name,
        format,
        bytes: out,
    })
}

/// Pack `document` as the one entry `entry_name` of a new ifcZIP archive,
/// Deflate-compressed.
///
/// The archive is reproducible: the entry carries the ZIP epoch
/// (1980-01-01) as its time, so the same document packs to the same bytes.
/// A document over 4 GiB gets a ZIP64 entry.
///
/// # Errors
///
/// [`IfcZipError::InvalidEntryName`] for a name [`extract`] would not take
/// as the model (not at the root, or not ending in `.ifc` or `.ifcXML`);
/// [`IfcZipError::UnknownInnerFormat`] for a document that is neither STEP
/// nor ifcXML; [`IfcZipError::Write`] if the archive cannot be written.
pub fn pack(entry_name: &str, document: &[u8]) -> Result<Vec<u8>, IfcZipError> {
    if !is_model_name(entry_name) {
        return Err(IfcZipError::InvalidEntryName(entry_name.to_owned()));
    }
    if InnerFormat::of_content(document).is_none() {
        return Err(IfcZipError::UnknownInnerFormat {
            name: entry_name.to_owned(),
        });
    }
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .large_file(document.len() as u64 >= u64::from(u32::MAX))
        .unix_permissions(0o644);
    let failed = |e: &dyn std::fmt::Display| IfcZipError::Write(e.to_string());
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(entry_name, options)
        .map_err(|e| failed(&e))?;
    writer.write_all(document).map_err(|e| failed(&e))?;
    Ok(writer.finish().map_err(|e| failed(&e))?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_entry_is_a_root_ifc_or_ifcxml_file() {
        assert!(is_model_name("model.ifc"));
        assert!(is_model_name("Model.IFCXML"));
        assert!(is_model_name("a.b.Ifc"));
        assert!(!is_model_name("dir/model.ifc"));
        assert!(!is_model_name("dir\\model.ifc"));
        assert!(!is_model_name("model.ifczip"));
        assert!(!is_model_name("model.xml"));
        assert!(!is_model_name("ifc"));
    }
}
