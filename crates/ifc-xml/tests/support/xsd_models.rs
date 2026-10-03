//! Hand-built IFC4 models that exercise every construct of the XSD
//! configuration the writer emits, shared by the round-trip tests
//! (`xsd_write.rs`) and the opt-in validation against `IFC4.xsd`
//! (`xsd_output.rs`).

use ifc_model::{Codec, EntityId, Model, Value};
use ifc_schema::{AggregateKind, Schema};
use std::collections::HashMap;

/// Spatial structure and relationships: every kind of inverse placement
/// (`IsDecomposedBy`, `ContainsElements`, `IsTypedBy`, `IsDefinedBy`,
/// `HasOpenings`, `HasFillings`, `HasSubContexts`, the direct
/// `HasCoordinateOperation` of an abstract declared type), typed `-wrapper`
/// values in SELECTs, an aggregate defined type of entities, references to
/// subtypes of an abstract declared type, enumerations, a list attribute,
/// derived slots, and strings that need escaping.
pub const SPATIAL: &str = "\
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',$,$,$,$,(#20),#30);
#2=IFCSITE('1YvctVUKr0kugbFTf53O9L',$,'Site',$,$,$,$,$,.ELEMENT.,(51,30,0,0),(7,0,0),100.5,$,$);
#3=IFCRELAGGREGATES('2YvctVUKr0kugbFTf53O9L',$,$,$,#1,(#2));
#4=IFCBUILDING('3YvctVUKr0kugbFTf53O9L',$,'B',$,$,$,$,$,.ELEMENT.,$,$,$);
#5=IFCRELAGGREGATES('4YvctVUKr0kugbFTf53O9L',$,$,$,#2,(#4));
#6=IFCWALL('5YvctVUKr0kugbFTf53O9L',$,'W & <wall> \"quoted\" ''single''',$,$,#52,$,$,.STANDARD.);
#7=IFCRELCONTAINEDINSPATIALSTRUCTURE('6YvctVUKr0kugbFTf53O9L',$,$,$,(#6,#12),#4);
#8=IFCWALLTYPE('7YvctVUKr0kugbFTf53O9L',$,'WT',$,$,$,$,$,$,.STANDARD.);
#9=IFCRELDEFINESBYTYPE('8YvctVUKr0kugbFTf53O9L',$,$,$,(#6),#8);
#10=IFCOPENINGELEMENT('9YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,.OPENING.);
#11=IFCRELVOIDSELEMENT('AYvctVUKr0kugbFTf53O9L',$,$,$,#6,#10);
#12=IFCDOOR('BYvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,2.1,0.9,.DOOR.,$,$);
#13=IFCRELFILLSELEMENT('CYvctVUKr0kugbFTf53O9L',$,$,$,#10,#12);
#14=IFCPROPERTYSINGLEVALUE('Label',$,IFCLABEL('x  y '),$);
#15=IFCPROPERTYSINGLEVALUE('Complex',$,IFCCOMPLEXNUMBER((1.5,-2.)),$);
#16=IFCPROPERTYSET('DYvctVUKr0kugbFTf53O9L',$,'Pset',$,(#14,#15));
#17=IFCRELDEFINESBYPROPERTIES('EYvctVUKr0kugbFTf53O9L',$,$,$,(#6,#12),#16);
#18=IFCRELDEFINESBYPROPERTIES('FYvctVUKr0kugbFTf53O9L',$,$,$,(#4),IFCPROPERTYSETDEFINITIONSET((#16)));
#20=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#22,$);
#21=IFCCARTESIANPOINT((0.,0.,0.));
#22=IFCAXIS2PLACEMENT3D(#21,$,$);
#23=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#20,$,.MODEL_VIEW.,$);
#30=IFCUNITASSIGNMENT((#31));
#31=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#40=IFCPROJECTEDCRS('EPSG:25832',$,$,$,$,$,$);
#41=IFCMAPCONVERSION(#20,#40,1.,2.,3.,$,$,$);
#50=IFCCARTESIANPOINT((1.25E+20,-3.5E-07));
#51=IFCAXIS2PLACEMENT3D(#21,$,$);
#52=IFCLOCALPLACEMENT($,#51);
";

/// Geometry and lists: flattened nested list attributes, a `Seq-` wrapped
/// nested list, a container of entities with `arraySize`, a container of
/// `-wrapper` values with `arraySize`, typed aggregates (`IfcLineIndex`,
/// `IfcArcIndex`) in a SELECT list, a binary element, the direct
/// `StyledByItem` inverse, a container of string wrappers, a string list
/// attribute and the logical `unknown`.
pub const GEOMETRY: &str = "\
#1=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.,0.,0.),(0.,1.,0.),(1.,1.,1.)));
#2=IFCINDEXEDPOLYGONALFACEWITHVOIDS((1,2,3),((2,3,4),(1,2,4,3)));
#3=IFCPOLYGONALFACESET(#1,$,(#2),$);
#4=IFCCARTESIANPOINTLIST2D(((0.,0.),(1.,0.),(1.,1.)));
#5=IFCINDEXEDPOLYCURVE(#4,(IFCLINEINDEX((1,2)),IFCARCINDEX((1,2,3))),.F.);
#6=IFCBSPLINESURFACEWITHKNOTS(1,1,((#10,#11),(#12,#13)),.UNSPECIFIED.,.F.,.F.,.F.,(2,2),(2,2),(0.,1.),(0.,1.),.UNSPECIFIED.);
#7=IFCRATIONALBSPLINESURFACEWITHKNOTS(1,1,((#10,#11),(#12,#13)),.UNSPECIFIED.,.F.,.F.,.F.,(2,2),(2,2),(0.,1.),(0.,1.),.UNSPECIFIED.,((1.,0.5),(0.5,1.)));
#10=IFCCARTESIANPOINT((0.,0.,0.));
#11=IFCCARTESIANPOINT((0.,1.,0.));
#12=IFCCARTESIANPOINT((1.,0.,0.));
#13=IFCCARTESIANPOINT((1.,1.,0.));
#20=IFCBLOBTEXTURE(.T.,.F.,'MODULATE',$,('a','b'),'PNG',\"089504E47\");
#21=IFCSTYLEDITEM(#3,(#22),'style');
#22=IFCSURFACESTYLE('S',.BOTH.,(#23));
#23=IFCSURFACESTYLESHADING(#24,$);
#24=IFCCOLOURRGB($,0.5,0.25,1.);
#30=IFCPOSTALADDRESS($,$,$,$,('Line 1','Line two'),$,'Town',$,$,$);
#31=IFCPERSON($,'Family','Given',('Middle','Other'),$,$,$,(#30));
#32=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#5);
#33=IFCCOMPOSITECURVE((#32),.U.);
";

/// A STEP document of one IFC4 data section.
pub fn step(data: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\n\
         FILE_NAME('model.ifc','2026-10-03T12:00:00+02:00',('Author'),('Organisation'),\
         'preprocessor','system','authorization');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
         {data}ENDSEC;\nEND-ISO-10303-21;\n"
    )
}

/// The model of an IFC4 data section.
pub fn model(data: &str) -> Model {
    ifc_step::StepCodec
        .read_bytes(step(data).as_bytes())
        .unwrap_or_else(|error| panic!("STEP read failed: {error}"))
}

/// Every construct model, named.
pub fn models() -> Vec<(&'static str, Model)> {
    vec![("spatial", model(SPATIAL)), ("geometry", model(GEOMETRY))]
}

/// Whether `read` is `source` renumbered in model order, sets as multisets.
pub fn same_model(schema: &Schema, source: &Model, read: &Model) -> Result<(), String> {
    let (expected, found) = (source.header(), read.header());
    let header = |model: &Model| {
        let header = model.header();
        (
            header.description.clone(),
            header.name.clone(),
            header.time_stamp.clone(),
            header.author.clone(),
            header.organization.clone(),
            header.preprocessor_version.clone(),
            header.originating_system.clone(),
            header.authorization.clone(),
            header.schema.clone(),
        )
    };
    if header(source) != header(read) {
        return Err(format!("header {found:?}, expected {expected:?}"));
    }
    if source.len() != read.len() {
        return Err(format!(
            "{} entities read back, {} written",
            read.len(),
            source.len()
        ));
    }
    let renumber: HashMap<EntityId, EntityId> = source
        .iter()
        .enumerate()
        .map(|(position, (id, _))| (id, EntityId(position as u64 + 1)))
        .collect();
    for (id, entity) in source.iter() {
        let new = renumber[&id];
        let other = read
            .get(new)
            .ok_or_else(|| format!("#{} (now #{}) missing", id.0, new.0))?;
        if !other.type_name.eq_ignore_ascii_case(&entity.type_name) {
            return Err(format!(
                "#{}: type {} read back as {}",
                id.0, entity.type_name, other.type_name
            ));
        }
        let attributes = schema.attributes(&entity.type_name);
        for (slot, (value, back)) in entity.attributes.iter().zip(&other.attributes).enumerate() {
            let unordered = attributes.get(slot).is_some_and(|attribute| {
                matches!(
                    attribute.aggregation.first().map(|level| level.kind),
                    Some(AggregateKind::Set | AggregateKind::Bag)
                )
            });
            let value = canonical(&renumbered(value, &renumber), unordered);
            let back = canonical(back, unordered);
            if value != back {
                return Err(format!(
                    "#{} {}.{}: {back} read back, {value} written",
                    id.0,
                    entity.type_name,
                    attributes
                        .get(slot)
                        .map_or("?", |attribute| &attribute.name)
                ));
            }
        }
        if entity.attributes.len() != other.attributes.len() {
            return Err(format!("#{}: attribute count differs", id.0));
        }
    }
    Ok(())
}

fn renumbered(value: &Value, renumber: &HashMap<EntityId, EntityId>) -> Value {
    match value {
        Value::Ref(id) => Value::Ref(renumber.get(id).copied().unwrap_or(*id)),
        Value::List(items) => Value::List(
            items
                .iter()
                .map(|item| renumbered(item, renumber))
                .collect(),
        ),
        Value::Typed { type_name, value } => Value::Typed {
            type_name: type_name.clone(),
            value: Box::new(renumbered(value, renumber)),
        },
        other => other.clone(),
    }
}

/// A comparable spelling: reals by their bits, an unordered top-level
/// aggregate sorted.
fn canonical(value: &Value, unordered: bool) -> String {
    fn spell(value: &Value) -> String {
        match value {
            Value::Real(real) => format!("Real({:#018x})", real.to_bits()),
            Value::List(items) => {
                let items: Vec<String> = items.iter().map(spell).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Typed { type_name, value } => format!("{type_name}({})", spell(value)),
            other => format!("{other:?}"),
        }
    }
    match value {
        Value::List(items) if unordered => {
            let mut items: Vec<String> = items.iter().map(spell).collect();
            items.sort();
            format!("set[{}]", items.join(", "))
        }
        other => spell(other),
    }
}
