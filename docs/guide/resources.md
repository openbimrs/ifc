# Construction resources

Enable the facade's `resource` feature to inspect and author the bounded IFC4
construction-resource slice:

```toml
openbim-ifc = { git = "https://github.com/openbimrs/ifc.git", features = ["resource"] }
```

The API selects IFC4 from `Model::header().schema`; IFC2X3, IFC4X3, missing,
and ambiguous declarations return typed errors. Attribute slots come from the
bundled IFC4 ADD2 TC1 schema rather than fixed indices.

```rust
use ifc::resource::{
    AllocationDraft, NestingDraft, ResourceDraft, ResourceEditor, ResourceKind,
    ResourceTimeDraft, ResourceView,
};
use ifc::Model;

let mut model = Model::new();
model.header_mut().schema.push("IFC4".into());

let (crew, carpenter, usage, allocation) = {
    let mut editor = ResourceEditor::for_model(&mut model)?;
    let usage = editor.create_time(
        ResourceTimeDraft::new()
            .name("Day shift")
            .schedule_work("PT8H")
            .schedule_usage(1.0),
    )?;
    let crew = editor.create_resource(
        ResourceDraft::new(ResourceKind::Crew, "2O2Fr$t4X7Zf8NOew3FLOH")
            .name("Envelope crew")
            .predefined_type("OFFICE")
            .usage(usage),
    )?;
    let carpenter = editor.create_resource(
        ResourceDraft::new(ResourceKind::Labor, "1O2Fr$t4X7Zf8NOew3FLOH")
            .name("Carpenter")
            .predefined_type("CARPENTRY"),
    )?;
    let allocation = editor.create_allocation(
        AllocationDraft::new("3O2Fr$t4X7Zf8NOew3FLOH", carpenter, vec![crew])
            .related_objects_type("RESOURCE"),
    )?;
    editor.create_nesting(NestingDraft::new(
        "0O2Fr$t4X7Zf8NOew3FLOH",
        crew,
        vec![carpenter],
    ))?;
    (crew, carpenter, usage, allocation)
};

let resources = ResourceView::for_model(&model)?;
assert_eq!(resources.allocation(allocation)?.related_objects_type(), Some("RESOURCE"));
assert_eq!(resources.resource(carpenter)?.name()?, Some("Carpenter"));
assert_eq!(resources.resource_time(usage)?.schedule_work()?, Some("PT8H"));
assert_eq!(resources.descendants(crew, Default::default())?, vec![carpenter]);
# Ok::<(), ifc::resource::ResourceError>(())
```

Invalid schema headers, references, enum members, ratios, aggregates, duplicate
`GlobalId` values, mismatched `IfcRelAssigns.RelatedObjectsType` categories,
second resource parents, cycle creation, malformed authored graphs, and exhausted
traversal budgets return typed `ResourceError` values.
Rejected drafts leave model length and revision unchanged.

## What this does not do

The crate preserves and validates authored resource data. It does **not** level
resources, calculate duration/cost/quantity, interpret work calendars, solve
logistics, or manufacture allocation from schedule/cost data. Actor, inventory,
construction-resource-type, IFC2X3, and IFC4X3 semantics are not yet public
capabilities.
