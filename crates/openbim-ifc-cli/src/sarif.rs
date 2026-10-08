//! SARIF 2.1.0 (OASIS) for `validate` and `lint`.
//!
//! One run per invocation: the tool, every file as an artifact, one result
//! per finding with the file and, for a STEP file, the line the entity's
//! record starts on, and the entity or attribute as a logical location
//! (`#12`, `#12.Name`). Files that could not be checked are tool execution
//! notifications, so a code-scanning upload still says the run failed.

use crate::json::Json;

/// Where the SARIF 2.1.0 JSON schema is published.
pub(crate) const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// Severity in SARIF's vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    /// Fails the run.
    Error,
    /// Worth a look.
    Warning,
    /// Informational.
    Note,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// One finding.
#[derive(Debug, Clone)]
pub(crate) struct Finding {
    /// Stable rule id.
    pub(crate) rule: String,
    /// Severity.
    pub(crate) level: Level,
    /// What is wrong.
    pub(crate) message: String,
    /// Index into the file list.
    pub(crate) file: usize,
    /// One-based line, when known.
    pub(crate) line: Option<usize>,
    /// `#12` or `#12.Name`; `None` for the file as a whole.
    pub(crate) entity: Option<String>,
    /// Whether [`Self::entity`] names an attribute (`member`) or an entity
    /// (`object`).
    pub(crate) is_attribute: bool,
}

/// A run: what was checked, what was found, what could not be checked.
#[derive(Debug, Default)]
pub(crate) struct Run {
    /// Every file named on the command line, as given.
    pub(crate) files: Vec<String>,
    /// Every finding.
    pub(crate) findings: Vec<Finding>,
    /// `(file index, reason)` for each file that could not be checked.
    pub(crate) failures: Vec<(usize, String)>,
}

impl Run {
    /// The SARIF log.
    pub(crate) fn to_json(&self) -> Json {
        let mut rules: Vec<&str> = Vec::new();
        for finding in &self.findings {
            if !rules.contains(&finding.rule.as_str()) {
                rules.push(&finding.rule);
            }
        }
        let driver = Json::object([
            ("name", Json::str("openbim-ifc")),
            ("version", Json::str(env!("CARGO_PKG_VERSION"))),
            ("semanticVersion", Json::str(env!("CARGO_PKG_VERSION"))),
            (
                "informationUri",
                Json::str("https://openbimrs.github.io/ifc/guide/cli"),
            ),
            (
                "rules",
                Json::Array(
                    rules
                        .iter()
                        .map(|rule| {
                            Json::object([
                                ("id", Json::str(*rule)),
                                (
                                    "shortDescription",
                                    Json::object([("text", Json::str(*rule))]),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]);
        let artifacts = self
            .files
            .iter()
            .map(|file| Json::object([("location", Json::object([("uri", Json::str(uri(file)))]))]))
            .collect();
        let results = self
            .findings
            .iter()
            .map(|finding| {
                let rule_index = rules
                    .iter()
                    .position(|rule| *rule == finding.rule)
                    .unwrap_or_default();
                let mut physical = vec![("artifactLocation", self.artifact(finding.file))];
                if let Some(line) = finding.line {
                    physical.push((
                        "region",
                        Json::object([("startLine", Json::uint(line as u64))]),
                    ));
                }
                let mut location = vec![("physicalLocation", Json::object(physical))];
                if let Some(entity) = &finding.entity {
                    location.push((
                        "logicalLocations",
                        Json::Array(vec![Json::object([
                            ("fullyQualifiedName", Json::str(entity.clone())),
                            (
                                "kind",
                                Json::str(if finding.is_attribute {
                                    "member"
                                } else {
                                    "object"
                                }),
                            ),
                        ])]),
                    ));
                }
                Json::object([
                    ("ruleId", Json::str(finding.rule.clone())),
                    ("ruleIndex", Json::uint(rule_index as u64)),
                    ("level", Json::str(finding.level.as_str())),
                    (
                        "message",
                        Json::object([("text", Json::str(finding.message.clone()))]),
                    ),
                    ("locations", Json::Array(vec![Json::object(location)])),
                ])
            })
            .collect();
        let notifications = self
            .failures
            .iter()
            .map(|(file, reason)| {
                Json::object([
                    ("level", Json::str("error")),
                    (
                        "message",
                        Json::object([("text", Json::str(reason.clone()))]),
                    ),
                    (
                        "locations",
                        Json::Array(vec![Json::object([(
                            "physicalLocation",
                            Json::object([("artifactLocation", self.artifact(*file))]),
                        )])]),
                    ),
                ])
            })
            .collect();
        let invocation = Json::object([
            ("executionSuccessful", Json::Bool(self.failures.is_empty())),
            ("toolExecutionNotifications", Json::Array(notifications)),
        ]);
        Json::object([
            ("$schema", Json::str(SCHEMA)),
            ("version", Json::str("2.1.0")),
            (
                "runs",
                Json::Array(vec![Json::object([
                    ("tool", Json::object([("driver", driver)])),
                    ("invocations", Json::Array(vec![invocation])),
                    ("artifacts", Json::Array(artifacts)),
                    ("results", Json::Array(results)),
                ])]),
            ),
        ])
    }

    fn artifact(&self, file: usize) -> Json {
        Json::object([
            ("uri", Json::str(uri(&self.files[file]))),
            ("index", Json::uint(file as u64)),
        ])
    }
}

/// A path as a URI reference: relative paths stay relative (the form
/// code-scanning uploads resolve against the checkout), absolute ones
/// become `file://` URIs; separators become `/` and anything outside the
/// unreserved set is percent-encoded.
pub(crate) fn uri(path: &str) -> String {
    let slashed = path.replace('\\', "/");
    let windows_drive = slashed.as_bytes().get(1) == Some(&b':')
        && slashed.as_bytes()[0].is_ascii_alphabetic()
        && slashed.as_bytes().get(2) == Some(&b'/');
    let mut out = String::new();
    if slashed.starts_with('/') {
        out.push_str("file://");
    } else if windows_drive {
        out.push_str("file:///");
    }
    for (index, byte) in slashed.bytes().enumerate() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
            || (windows_drive && index == 1);
        if keep {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_become_uri_references() {
        assert_eq!(uri("models/a b.ifc"), "models/a%20b.ifc");
        assert_eq!(uri("/srv/x.ifc"), "file:///srv/x.ifc");
        assert_eq!(uri("C:\\m\\x.ifc"), "file:///C:/m/x.ifc");
    }
}
