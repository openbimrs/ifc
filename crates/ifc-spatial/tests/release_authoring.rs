//! Spatial authoring bound to the declared release round-trips (#202).
//!
//! Every record the `*_with_owner_history` writers can stage in IFC2X3,
//! IFC4 and IFC4X3 is written to STEP, read back with `ifc-step`, checked
//! slot by slot against the release's own table, and read back through the
//! crate's views.

mod release_fixture;

use ifc_model::{Codec, Value};
use ifc_schema::{for_version, SchemaVersion};
use ifc_spatial::relation::{self, boundary, RelationshipKind};
use ifc_spatial::SpatialTree;
use ifc_step::StepCodec;
use release_fixture::{author, base, OWNER, RELEASES, SPACE, WALL, WALL2};

#[test]
fn every_spatial_record_round_trips_in_its_release() {
    for (schema, version) in RELEASES {
        let table = for_version(version).unwrap();
        let mut model = base(schema, version);
        let authored = author(&mut model, version);
        let expected = match version {
            SchemaVersion::Ifc2x3 => 22,
            SchemaVersion::Ifc4 => 32,
            SchemaVersion::Ifc4x3 => 36,
            other => unreachable!("{other:?} is not swept"),
        };
        assert_eq!(authored.written.len(), expected, "{schema}");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for id in &authored.written {
            let record = back.get(*id).expect("read back");
            let names = table.attribute_names(&record.type_name);
            assert_eq!(
                record.attributes.len(),
                names.len(),
                "{schema} {}",
                record.type_name
            );
            let owner = names.iter().position(|n| *n == "OwnerHistory").unwrap();
            assert_eq!(record.attributes[owner], Value::Ref(OWNER), "{schema}");
        }

        // IFC2X3 names the covered space `RelatedSpace` (IFC4 renamed it).
        let covers = back.get(authored.covers_spaces).unwrap();
        let names = table.attribute_names("IFCRELCOVERSSPACES");
        let space_name = if version == SchemaVersion::Ifc2x3 {
            "RelatedSpace"
        } else {
            "RelatingSpace"
        };
        let slot = names.iter().position(|n| *n == space_name).unwrap();
        assert_eq!(covers.attributes[slot], Value::Ref(SPACE), "{schema}");

        // The crate's own views read what was written.
        let tree = SpatialTree::build(&back);
        assert_eq!(tree.release(), Some(version), "{schema}");
        assert_eq!(tree.container_of(WALL), Some(authored.storey), "{schema}");
        assert_eq!(tree.elements_of(authored.storey), [WALL, WALL2], "{schema}");
        let ancestors = tree.ancestors(authored.storey);
        for parent in [authored.building, authored.site, authored.project] {
            assert!(ancestors.contains(&parent), "{schema}: {ancestors:?}");
        }
        let relationships = relation::all(&back);
        let covering = relationships
            .iter()
            .find(|r| r.id == authored.covers_spaces)
            .expect("covers spaces");
        assert_eq!(covering.kind, RelationshipKind::CoversSpaces);
        assert_eq!(covering.relating, Some(SPACE), "{schema}");
        let contained = relationships
            .iter()
            .find(|r| r.id == authored.contained)
            .expect("contained");
        assert_eq!(contained.relating, Some(authored.storey));
        let boundaries = boundary::of_space(&back, SPACE);
        let written = boundaries
            .iter()
            .find(|b| b.id == authored.boundary)
            .expect("boundary");
        assert_eq!(written.element, Some(WALL), "{schema}");
        assert_eq!(written.physical_matches_element(&back), Some(true));
    }
}
