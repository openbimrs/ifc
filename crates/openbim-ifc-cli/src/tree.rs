//! `tree`: the spatial structure, from the project down.
//!
//! The facade's `SpatialTree` reads the aggregation, containment and
//! spatial-reference relationships once, classifying containers against
//! the declared release. What it could not honour (a second parent, a
//! containment in something that is no container, a relationship naming an
//! entity the file lacks, a detached branch) is printed after the tree,
//! never dropped.

use std::collections::BTreeSet;
use std::io::Write;

use ifc::spatial::SpatialAnomaly;
use ifc::{EntityId, Model, Schema, SpatialKind, SpatialTree};

use crate::cli::{TextFormat, TreeArgs};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;

/// Run `tree`.
pub(crate) fn run(args: &TreeArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    let schema = input::declared_schema(model)?;
    let tree = SpatialTree::build(model);
    let view = View {
        model,
        schema,
        tree: &tree,
    };
    let written = match args.format {
        TextFormat::Human => out.write_all(view.human(args.elements).as_bytes()),
        TextFormat::Json => out.write_all(view.json().pretty().as_bytes()),
    };
    written.map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

struct View<'a> {
    model: &'a Model,
    schema: &'a Schema,
    tree: &'a SpatialTree,
}

impl View<'_> {
    /// `IFCWALL #36 'Wall'`.
    fn label(&self, id: EntityId) -> String {
        let type_name = self
            .model
            .get(id)
            .map(|entity| entity.type_name.to_string())
            .unwrap_or_else(|| "(missing)".to_owned());
        match self.name(id) {
            Some(name) => format!("{type_name} #{} '{name}'", id.0),
            None => format!("{type_name} #{}", id.0),
        }
    }

    fn name(&self, id: EntityId) -> Option<String> {
        ifc::root_identity(self.model, self.schema, id)
            .and_then(|identity| identity.name.map(str::to_owned))
    }

    fn human(&self, elements: bool) -> String {
        let mut text = String::new();
        let mut seen = BTreeSet::new();
        for root in self.tree.roots() {
            text.push_str(&self.label(*root));
            text.push_str(&self.element_note(*root, elements));
            text.push('\n');
            seen.insert(*root);
            self.children(*root, "", elements, &mut seen, &mut text);
        }
        if self.tree.roots().is_empty() {
            text.push_str("no spatial structure\n");
        }
        let unreached: Vec<EntityId> = self
            .tree
            .containers()
            .map(|node| node.id)
            .filter(|id| !seen.contains(id))
            .collect();
        let mut notes = Vec::new();
        for id in &unreached {
            notes.push(format!("{} is not reachable from a root", self.label(*id)));
        }
        if self.tree.roots().len() > 1 {
            notes.push(format!(
                "{} roots: a conformant file has one IfcProject",
                self.tree.roots().len()
            ));
        }
        for orphan in self.tree.orphans() {
            notes.push(format!(
                "{} is aggregated under nothing (detached branch)",
                self.label(*orphan)
            ));
        }
        for (relation, target) in self.tree.dangling() {
            notes.push(format!(
                "#{} names #{}, which is not in the file",
                relation.0, target.0
            ));
        }
        for anomaly in self.tree.anomalies() {
            notes.push(anomaly_text(anomaly));
        }
        if !notes.is_empty() {
            text.push_str("\nAnomalies:\n");
            for note in notes {
                text.push_str(&format!("  {note}\n"));
            }
        }
        text
    }

    fn element_note(&self, id: EntityId, listed: bool) -> String {
        let count = self.tree.elements_of(id).len();
        let referenced = self.tree.referenced_elements(id).len();
        let mut note = String::new();
        if count > 0 && !listed {
            note.push_str(&format!(
                "  ({count} element{})",
                if count == 1 { "" } else { "s" }
            ));
        }
        if referenced > 0 {
            note.push_str(&format!("  ({referenced} referenced)"));
        }
        note
    }

    fn children(
        &self,
        id: EntityId,
        prefix: &str,
        elements: bool,
        seen: &mut BTreeSet<EntityId>,
        text: &mut String,
    ) {
        let Some(node) = self.tree.node(id) else {
            return;
        };
        let listed: &[EntityId] = if elements { &node.elements } else { &[] };
        let total = listed.len() + node.children.len();
        let mut index = 0;
        for element in listed {
            index += 1;
            let branch = if index == total { "└─ " } else { "├─ " };
            text.push_str(&format!("{prefix}{branch}· {}\n", self.label(*element)));
        }
        for child in &node.children {
            index += 1;
            let last = index == total;
            let branch = if last { "└─ " } else { "├─ " };
            text.push_str(&format!(
                "{prefix}{branch}{}{}\n",
                self.label(*child),
                self.element_note(*child, elements)
            ));
            // A container met twice would be a cycle; the tree keeps one
            // parent per container, so this only guards a malformed file.
            if seen.insert(*child) {
                let deeper = format!("{prefix}{}", if last { "   " } else { "│  " });
                self.children(*child, &deeper, elements, seen, text);
            }
        }
    }

    fn json(&self) -> Json {
        let mut seen = BTreeSet::new();
        let roots = self
            .tree
            .roots()
            .iter()
            .map(|root| self.node_json(*root, &mut seen))
            .collect();
        let ids = |ids: &[EntityId]| Json::Array(ids.iter().map(|id| Json::uint(id.0)).collect());
        Json::object([
            (
                "release",
                Json::opt_str(self.tree.release().map(|release| release.release_id())),
            ),
            ("roots", Json::Array(roots)),
            ("orphans", ids(self.tree.orphans())),
            (
                "dangling",
                Json::Array(
                    self.tree
                        .dangling()
                        .iter()
                        .map(|(relation, target)| {
                            Json::object([
                                ("relation", Json::uint(relation.0)),
                                ("target", Json::uint(target.0)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "anomalies",
                Json::Array(
                    self.tree
                        .anomalies()
                        .iter()
                        .map(|anomaly| Json::str(anomaly_text(anomaly)))
                        .collect(),
                ),
            ),
        ])
    }

    fn entity_json(&self, id: EntityId) -> Vec<(&'static str, Json)> {
        let identity = ifc::root_identity(self.model, self.schema, id);
        vec![
            ("id", Json::uint(id.0)),
            (
                "type",
                Json::opt_str(
                    self.model
                        .get(id)
                        .map(|entity| entity.type_name.to_string()),
                ),
            ),
            (
                "global_id",
                Json::opt_str(identity.and_then(|identity| identity.global_id)),
            ),
            (
                "name",
                Json::opt_str(identity.and_then(|identity| identity.name)),
            ),
        ]
    }

    fn node_json(&self, id: EntityId, seen: &mut BTreeSet<EntityId>) -> Json {
        seen.insert(id);
        let mut fields = self.entity_json(id);
        let Some(node) = self.tree.node(id) else {
            return Json::object(fields);
        };
        fields.insert(2, ("kind", Json::str(kind_name(node.kind))));
        let entities = |ids: &[EntityId]| {
            Json::Array(
                ids.iter()
                    .map(|id| Json::object(self.entity_json(*id)))
                    .collect(),
            )
        };
        fields.push(("elements", entities(&node.elements)));
        fields.push(("referenced", entities(self.tree.referenced_elements(id))));
        let children = node
            .children
            .iter()
            .filter(|child| !seen.contains(child))
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|child| self.node_json(child, seen))
            .collect();
        fields.push(("children", Json::Array(children)));
        Json::object(fields)
    }
}

fn kind_name(kind: SpatialKind) -> &'static str {
    match kind {
        SpatialKind::Project => "project",
        SpatialKind::Site => "site",
        SpatialKind::Building => "building",
        SpatialKind::Storey => "storey",
        SpatialKind::Space => "space",
        _ => "other",
    }
}

fn anomaly_text(anomaly: &SpatialAnomaly) -> String {
    match anomaly {
        SpatialAnomaly::ContainedTwice {
            element,
            kept,
            relation,
            ..
        } => format!(
            "#{} is contained twice: #{} ignored, #{} kept",
            element.0, relation.0, kept.0
        ),
        SpatialAnomaly::AggregatedTwice {
            child,
            kept,
            relation,
            ..
        } => format!(
            "#{} is aggregated twice: #{} ignored, #{} kept",
            child.0, relation.0, kept.0
        ),
        SpatialAnomaly::ContainedInNonContainer {
            relation,
            structure,
        } => format!(
            "#{} contains elements in #{}, which is no spatial container",
            relation.0, structure.0
        ),
        SpatialAnomaly::ReferencedInNonContainer {
            relation,
            structure,
        } => format!(
            "#{} references elements in #{}, which is no spatial container",
            relation.0, structure.0
        ),
        other => format!("{other:?}"),
    }
}
