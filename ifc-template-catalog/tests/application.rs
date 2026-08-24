#[path = "support/mod.rs"]
mod support;

use ifc_template_catalog::compliance::{apply_template, TemplateSink};
use ifc_template_catalog::definition::{
    PropertyDataType, PropertyKind, PropertyTemplate, SetTemplate, SetTemplateKind,
};

#[derive(Default)]
struct Sink(Vec<String>);
impl TemplateSink for Sink {
    type Error = ();
    fn begin(&mut self, template: &SetTemplate) -> Result<(), Self::Error> {
        self.0.push(template.name.clone());
        Ok(())
    }
    fn property(&mut self, path: &str, _: &PropertyTemplate) -> Result<(), Self::Error> {
        self.0.push(path.into());
        Ok(())
    }
    fn finish(&mut self, _: &SetTemplate) -> Result<(), Self::Error> {
        self.0.push("done".into());
        Ok(())
    }
}

#[test]
fn application_walks_typed_members_through_a_sink() {
    let mut template = support::property_set("Pset_Test");
    let SetTemplateKind::Property { properties, .. } = &mut template.kind else {
        panic!()
    };
    properties.push(PropertyTemplate {
        name: "Enabled".into(),
        guid: None,
        definition: None,
        name_aliases: vec![],
        definition_aliases: vec![],
        kind: PropertyKind::SingleValue {
            data_type: PropertyDataType::new("IfcBoolean"),
        },
    });
    let mut sink = Sink::default();
    apply_template(&template, &mut sink).unwrap();
    assert_eq!(sink.0, ["Pset_Test", "Enabled", "done"]);
}
