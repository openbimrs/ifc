//! JavaScript surface of the shared [`Core`] model.
//!
//! Each export converts JS arguments, calls the core method, and converts
//! the result. No IFC logic lives here.

use js_sys::{Array, BigInt, Uint8Array};
use openbim_ifc_binding_core::record::to_records;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{BindingError, IfcModel as Core, ToRecord};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::authoring;
use crate::edits;
use crate::error::js_error;
use crate::geometry;
use crate::records;
use crate::value::{from_js, to_js};

mod domain_types;
mod geometry_types;
mod types;

/// An IFC model: entities keyed by their `#id`, in file order.
///
/// A newtype because `wasm_bindgen` can only export a type defined in this
/// crate; all state and behaviour are the core's.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct IfcModel(Core);

fn index(value: u32) -> usize {
    // wasm32 has a 32-bit usize, so a u32 always fits.
    value as usize
}

fn big(id: u64) -> JsValue {
    BigInt::from(id).into()
}

fn tagged_array(values: &JsValue) -> Result<Vec<Tagged>, BindingError> {
    if !Array::is_array(values) {
        return Err(BindingError::InvalidValue(
            "attributes must be an array".into(),
        ));
    }
    Array::from(values).iter().map(|v| from_js(&v)).collect()
}

#[wasm_bindgen]
impl IfcModel {
    /// An empty model.
    #[wasm_bindgen(constructor)]
    pub fn new() -> IfcModel {
        IfcModel(Core::empty())
    }

    /// Parse a STEP (`.ifc`) file from its bytes.
    #[wasm_bindgen(js_name = parse)]
    pub fn parse_js(bytes: &[u8]) -> Result<IfcModel, JsValue> {
        Core::parse(bytes).map(IfcModel).map_err(js_error)
    }

    /// Parse a STEP file under explicit read options. With `onMalformed:
    /// "skip"` a damaged record is dropped and reported by `diagnostics()`
    /// instead of failing the read; omitted options are strict.
    #[wasm_bindgen(js_name = parseWithOptions)]
    pub fn parse_with_options_js(
        bytes: &[u8],
        #[wasm_bindgen(unchecked_param_type = "ParseOptions")] options: &JsValue,
    ) -> Result<IfcModel, JsValue> {
        let options = records::parse_options(options).map_err(js_error)?;
        Core::parse_with(bytes, options)
            .map(IfcModel)
            .map_err(js_error)
    }

    /// Parse an ifcXML document: the library's lossless layout without a
    /// profile, or the buildingSMART XSD layout of `"IFC4"` or
    /// `"IFC4X3_ADD2"`.
    #[wasm_bindgen(js_name = parseIfcXml)]
    pub fn parse_ifcxml_js(bytes: &[u8], profile: Option<String>) -> Result<IfcModel, JsValue> {
        Core::parse_ifcxml(bytes, profile.as_deref())
            .map(IfcModel)
            .map_err(js_error)
    }

    /// Serialize as STEP bytes.
    #[wasm_bindgen(js_name = write)]
    pub fn write_js(&self) -> Result<Uint8Array, JsValue> {
        Ok(Uint8Array::from(
            self.0.write().map_err(js_error)?.as_slice(),
        ))
    }

    /// Serialize as ifcXML bytes, in the layout `parseIfcXml` reads.
    #[wasm_bindgen(js_name = writeIfcXml)]
    pub fn write_ifcxml_js(&self, profile: Option<String>) -> Result<Uint8Array, JsValue> {
        Ok(Uint8Array::from(
            self.0
                .write_ifcxml(profile.as_deref())
                .map_err(js_error)?
                .as_slice(),
        ))
    }

    /// The STEP file header: description, name, time stamp, author,
    /// organization, preprocessor, originating system, authorization and
    /// schema tokens.
    #[wasm_bindgen(js_name = header, unchecked_return_type = "IfcHeader")]
    pub fn header_js(&self) -> JsValue {
        records::header_to_js(&self.0.header())
    }

    /// Replace the STEP file header; every field is required.
    #[wasm_bindgen(js_name = setHeader)]
    pub fn set_header_js(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "IfcHeader")] header: &JsValue,
    ) -> Result<(), JsValue> {
        let header = records::header_from_js(header).map_err(js_error)?;
        self.0.set_header(header);
        Ok(())
    }

    /// Validate against the schema the header declares; findings are
    /// sorted by severity, rule, entity and slot. `maxFindings` caps the
    /// report (default 10,000) and sets `truncated` when reached.
    #[wasm_bindgen(js_name = validate, unchecked_return_type = "ValidationReport")]
    pub fn validate_js(
        &self,
        #[wasm_bindgen(js_name = maxFindings)] max_findings: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let report = self.0.validate(max_findings.map(index)).map_err(js_error)?;
        Ok(records::report_to_js(&report))
    }

    /// Products no viewer will draw (outside the spatial structure, or with
    /// geometry only in non-model contexts), with a stable `reason`.
    #[wasm_bindgen(js_name = unreachableProducts, unchecked_return_type = "UnreachableProduct[]")]
    pub fn unreachable_products_js(&self) -> Result<Array, JsValue> {
        Ok(records::unreachable_to_js(
            &self.0.unreachable_products().map_err(js_error)?,
        ))
    }

    /// Each product's world placement (a column-major 4x4 in metres) and
    /// the Body representation a viewer draws, for `ids` or, without, for
    /// every product with a shape. A product that cannot be placed is a
    /// record with a typed `refusal`; the call itself throws only
    /// `unsupported-schema` or `feature-disabled` (feature `placements`).
    #[wasm_bindgen(js_name = productPlacements, unchecked_return_type = "ProductPlacement[]")]
    pub fn product_placements_js(
        &self,
        #[wasm_bindgen(unchecked_param_type = "bigint[] | BigUint64Array | undefined")]
        ids: &JsValue,
    ) -> Result<Array, JsValue> {
        let ids = geometry::ids(ids).map_err(js_error)?;
        let placements = self
            .0
            .product_placements(ids.as_deref())
            .map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&placements)))
    }

    /// Each product's Body as triangles from the reference backend:
    /// `positions` (`Float32Array`, metres, relative to `transform`) and
    /// `indices` (`Uint32Array`), for `ids` or, without, every product
    /// with a shape. A product that cannot be meshed has a typed
    /// `refusal`. Opt-in: a build without the `mesh` feature (the npm
    /// package) throws `feature-disabled`.
    #[wasm_bindgen(js_name = productMeshes, unchecked_return_type = "ProductMesh[]")]
    pub fn product_meshes_js(
        &self,
        #[wasm_bindgen(unchecked_param_type = "bigint[] | BigUint64Array | undefined")]
        ids: &JsValue,
    ) -> Result<Array, JsValue> {
        let ids = geometry::ids(ids).map_err(js_error)?;
        let meshes = self.0.product_meshes(ids.as_deref()).map_err(js_error)?;
        Ok(geometry::meshes_to_js(&meshes))
    }

    /// The property sets, quantity sets and predefined property sets that
    /// apply to object `id`: its own first, then those inherited from its
    /// type object, an occurrence property overriding an inherited one.
    /// Values keep their declared IFC type (`typed IFCLENGTHMEASURE(...)`).
    #[wasm_bindgen(js_name = propertySets, unchecked_return_type = "PropertySet[]")]
    pub fn property_sets_js(&self, id: u64) -> Result<Array, JsValue> {
        let sets = self.0.property_sets(id).map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&sets)))
    }

    /// The effective unit of a `measureType` value (`"IFCAREAMEASURE"`):
    /// `unit` when given (a property's stated unit), otherwise the project
    /// default, resolved exactly to SI.
    #[wasm_bindgen(js_name = resolveUnit, unchecked_return_type = "ResolvedUnit")]
    pub fn resolve_unit_js(
        &self,
        #[wasm_bindgen(js_name = measureType)] measure_type: &str,
        #[wasm_bindgen(unchecked_param_type = "bigint | undefined")] unit: Option<u64>,
    ) -> Result<JsValue, JsValue> {
        let unit = self.0.resolve_unit(measure_type, unit).map_err(js_error)?;
        Ok(records::record_to_js(&unit.to_record()))
    }

    /// The spatial containment tree: every container with its parent,
    /// sub-containers and contained elements.
    #[wasm_bindgen(js_name = spatialTree, unchecked_return_type = "SpatialTree")]
    pub fn spatial_tree_js(&self) -> Result<JsValue, JsValue> {
        let tree = self.0.spatial_tree().map_err(js_error)?;
        Ok(records::record_to_js(&tree.to_record()))
    }

    /// The classifications that apply to object `id`: its own, then its
    /// type object's.
    #[wasm_bindgen(js_name = classifications, unchecked_return_type = "Classification[]")]
    pub fn classifications_js(&self, id: u64) -> Result<Array, JsValue> {
        let classes = self.0.classifications(id).map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&classes)))
    }

    /// The material association that applies to object `id` (its own, or
    /// its type object's), or `undefined` when there is none.
    #[wasm_bindgen(js_name = material, unchecked_return_type = "MaterialAssignment | undefined")]
    pub fn material_js(&self, id: u64) -> Result<JsValue, JsValue> {
        let material = self.0.material(id).map_err(js_error)?;
        Ok(material.map_or(JsValue::UNDEFINED, |material| {
            records::record_to_js(&material.to_record())
        }))
    }

    /// Every system with its members and served structures, and the
    /// memberships the reader could not honour.
    #[wasm_bindgen(js_name = systems, unchecked_return_type = "Systems")]
    pub fn systems_js(&self) -> Result<JsValue, JsValue> {
        let systems = self.0.systems().map_err(js_error)?;
        Ok(records::record_to_js(&systems.to_record()))
    }

    /// Every cost schedule and cost item, with values in the tagged
    /// encoding.
    #[wasm_bindgen(js_name = cost, unchecked_return_type = "Cost")]
    pub fn cost_js(&self) -> Result<JsValue, JsValue> {
        let cost = self.0.cost().map_err(js_error)?;
        Ok(records::record_to_js(&cost.to_record()))
    }

    /// Every coordinate operation (map conversion) resolved with the
    /// project length unit; empty when the model has none.
    #[wasm_bindgen(js_name = georeferencing, unchecked_return_type = "MapConversion[]")]
    pub fn georeferencing_js(&self) -> Result<Array, JsValue> {
        let maps = self.0.georeferencing().map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&maps)))
    }

    /// Write and remove property and quantity values as one checked
    /// transaction: every edit, in order, or none, and a refused batch
    /// leaves the model unchanged. A write's `value` is the read side's
    /// `value`; an inherited value is overridden on the occurrence, never
    /// changed on the shared type set.
    #[wasm_bindgen(js_name = setProperties, unchecked_return_type = "PropertyEditResult")]
    pub fn set_properties_js(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "PropertyEdit[]")] edits: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let edits = edits::edits_from_js(edits).map_err(js_error)?;
        let result = self.0.set_properties(edits).map_err(js_error)?;
        Ok(records::record_to_js(&result.to_record()))
    }

    /// Write one value (`setProperties` with one edit); returns the entity
    /// holding it (`bigint`).
    #[wasm_bindgen(js_name = setProperty)]
    pub fn set_property_js(
        &mut self,
        object: u64,
        set: &str,
        name: &str,
        #[wasm_bindgen(unchecked_param_type = "IfcValue")] value: &JsValue,
        #[wasm_bindgen(js_name = setType)] set_type: Option<String>,
    ) -> Result<u64, JsValue> {
        let value = from_js(value).map_err(js_error)?;
        self.0
            .set_property(object, set, name, value, set_type)
            .map_err(js_error)
    }

    /// The file name of `release`'s PSD/QTO catalog snapshot, such as
    /// `"ifc4x3-add2.bin"`; the package ships it as `catalog/<name>`.
    /// `release` is a header schema token: `"IFC2X3"`, `"IFC4"` or
    /// `"IFC4X3"` (`"IFC4X3_ADD2"`).
    #[wasm_bindgen(js_name = catalogFile)]
    pub fn catalog_file_js(release: &str) -> Result<String, JsValue> {
        openbim_ifc_binding_core::catalog::file_name(release)
            .map(str::to_owned)
            .map_err(js_error)
    }

    /// Whether `release`'s catalog is loaded in this module instance, so a
    /// write to its `Pset_`/`Qto_` sets can be checked.
    #[wasm_bindgen(js_name = catalogLoaded)]
    pub fn catalog_loaded_js(release: &str) -> Result<bool, JsValue> {
        openbim_ifc_binding_core::catalog::is_loaded(release).map_err(js_error)
    }

    /// Load `release`'s catalog from the bytes of its snapshot file, checked
    /// against the pinned SHA-256 (`invalid-value` otherwise). The
    /// synchronous half of `IfcModel.loadCatalog`, for a host that reads
    /// the file itself.
    #[wasm_bindgen(js_name = loadCatalogBytes)]
    pub fn load_catalog_bytes_js(release: &str, bytes: &[u8]) -> Result<(), JsValue> {
        openbim_ifc_binding_core::catalog::load(release, bytes).map_err(js_error)
    }

    /// Remove one property from the object's own set (`setProperties` with
    /// one edit).
    #[wasm_bindgen(js_name = removeProperty)]
    pub fn remove_property_js(
        &mut self,
        object: u64,
        set: &str,
        name: &str,
    ) -> Result<(), JsValue> {
        self.0.remove_property(object, set, name).map_err(js_error)
    }

    /// Number of entities.
    #[wasm_bindgen(getter, js_name = size)]
    pub fn size_js(&self) -> u32 {
        u32::try_from(self.0.len()).unwrap_or(u32::MAX)
    }

    /// The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `undefined`.
    #[wasm_bindgen(getter, js_name = schema)]
    pub fn schema_js(&self) -> Option<String> {
        self.0.schema().map(str::to_owned)
    }

    /// Non-fatal problems found while reading.
    #[wasm_bindgen(js_name = diagnostics)]
    pub fn diagnostics_js(&self) -> Vec<String> {
        self.0.diagnostics()
    }

    /// Every entity id (`bigint`), in file order.
    #[wasm_bindgen(js_name = ids, unchecked_return_type = "bigint[]")]
    pub fn ids_js(&self) -> Array {
        self.0.ids().into_iter().map(big).collect()
    }

    /// Ids of every entity of exactly `typeName`, case-insensitive.
    #[wasm_bindgen(js_name = idsOfType, unchecked_return_type = "bigint[]")]
    pub fn ids_of_type_js(&self, #[wasm_bindgen(js_name = typeName)] type_name: &str) -> Array {
        self.0.ids_of_type(type_name).into_iter().map(big).collect()
    }

    /// Ids of every entity of `typeName` or any subtype, per the file's
    /// declared schema: `IfcWall` also finds `IFCWALLSTANDARDCASE`.
    #[wasm_bindgen(js_name = idsOfTypeIncludingSubtypes, unchecked_return_type = "bigint[]")]
    pub fn ids_of_type_including_subtypes_js(
        &self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
    ) -> Result<Array, JsValue> {
        Ok(self
            .0
            .ids_of_type_including_subtypes(type_name)
            .map_err(js_error)?
            .into_iter()
            .map(big)
            .collect())
    }

    /// The upper-case type name of entity `id`.
    #[wasm_bindgen(js_name = typeOf)]
    pub fn type_of_js(&self, id: u64) -> Result<String, JsValue> {
        Ok(self.0.type_of(id).map_err(js_error)?.to_owned())
    }

    /// Every attribute of entity `id`, as tagged values.
    #[wasm_bindgen(js_name = attributes, unchecked_return_type = "IfcValue[]")]
    pub fn attributes_js(&self, id: u64) -> Result<Array, JsValue> {
        Ok(self
            .0
            .attributes(id)
            .map_err(js_error)?
            .iter()
            .map(to_js)
            .collect())
    }

    /// Attribute `index` of entity `id`, as a tagged value.
    #[wasm_bindgen(js_name = attribute, unchecked_return_type = "IfcValue")]
    pub fn attribute_js(&self, id: u64, slot: u32) -> Result<JsValue, JsValue> {
        Ok(to_js(&self.0.attribute(id, index(slot)).map_err(js_error)?))
    }

    /// Set attribute `index` of entity `id`; returns the previous value.
    #[wasm_bindgen(js_name = setAttribute, unchecked_return_type = "IfcValue")]
    pub fn set_attribute_js(
        &mut self,
        id: u64,
        slot: u32,
        #[wasm_bindgen(unchecked_param_type = "IfcValue")] value: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let value = from_js(value).map_err(js_error)?;
        Ok(to_js(
            &self
                .0
                .set_attribute(id, index(slot), value)
                .map_err(js_error)?,
        ))
    }

    /// Every explicit attribute of entity `id` in slot order, inherited
    /// first, as the release the header declares defines them.
    #[wasm_bindgen(js_name = attributeNames, unchecked_return_type = "AttributeInfo[]")]
    pub fn attribute_names_js(&self, id: u64) -> Result<Array, JsValue> {
        let names = self.0.attribute_names(id).map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&names)))
    }

    /// Attribute `name` of entity `id` (case-insensitive, e.g. `"Name"`),
    /// resolved against the declared release, as a tagged value.
    #[wasm_bindgen(js_name = attributeByName, unchecked_return_type = "IfcValue")]
    pub fn attribute_by_name_js(&self, id: u64, name: &str) -> Result<JsValue, JsValue> {
        Ok(to_js(
            &self.0.attribute_by_name(id, name).map_err(js_error)?,
        ))
    }

    /// Set attribute `name` of entity `id`; returns the previous value. A
    /// derived attribute is refused (`derived-attribute`).
    #[wasm_bindgen(js_name = setAttributeByName, unchecked_return_type = "IfcValue")]
    pub fn set_attribute_by_name_js(
        &mut self,
        id: u64,
        name: &str,
        #[wasm_bindgen(unchecked_param_type = "IfcValue")] value: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let value = from_js(value).map_err(js_error)?;
        Ok(to_js(
            &self
                .0
                .set_attribute_by_name(id, name, value)
                .map_err(js_error)?,
        ))
    }

    /// Append an entity; returns its id (`bigint`).
    #[wasm_bindgen(js_name = add)]
    pub fn add_js(
        &mut self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
        #[wasm_bindgen(unchecked_param_type = "IfcValue[]")] attributes: &JsValue,
    ) -> Result<u64, JsValue> {
        let attributes = tagged_array(attributes).map_err(js_error)?;
        self.0.add(type_name, attributes).map_err(js_error)
    }

    /// Remove entity `id`, leaving references to it dangling.
    #[wasm_bindgen(js_name = remove)]
    pub fn remove_js(&mut self, id: u64) -> Result<(), JsValue> {
        self.0.remove(id).map_err(js_error)
    }

    /// Apply authoring operations as one checked transaction against the
    /// release the header declares: every operation, in order, or none, and
    /// a refused batch leaves the model unchanged. An operation names the
    /// entity an earlier one produced by `IfcModel.handle(index)`.
    /// `result.ids` holds, per operation, the id of the entity it produced.
    #[wasm_bindgen(js_name = author, unchecked_return_type = "AuthoringResult")]
    pub fn author_js(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "AuthorOp[]")] ops: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let ops = authoring::ops_from_js(ops).map_err(js_error)?;
        let result = self
            .0
            .author_seeded(ops, authoring::seed())
            .map_err(js_error)?;
        Ok(records::record_to_js(&result.to_record()))
    }

    /// Create one entity of `typeName` from named attributes, checked
    /// against the declared release (`author` with one `create`); an
    /// `IfcRoot` without a `GlobalId` gets one. Returns its id (`bigint`).
    #[wasm_bindgen(js_name = createEntity)]
    pub fn create_entity_js(
        &mut self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
        #[wasm_bindgen(unchecked_param_type = "Record<string, IfcValue>")] attributes: &JsValue,
    ) -> Result<u64, JsValue> {
        let attributes = if attributes.is_undefined() || attributes.is_null() {
            Tagged::List(Vec::new())
        } else {
            authoring::attributes(attributes).map_err(js_error)?
        };
        let op = authoring::single(
            "create",
            vec![
                ("type", Tagged::Text(type_name.to_owned())),
                ("attributes", attributes),
            ],
        )
        .map_err(js_error)?;
        let result = self
            .0
            .author_seeded(vec![op], authoring::seed())
            .map_err(js_error)?;
        result
            .ids
            .first()
            .copied()
            .flatten()
            .ok_or_else(|| js_error(BindingError::InvalidModel("nothing was created".into())))
    }

    /// Remove entity `id` with its relationships, leaving nothing dangling
    /// (`author` with one `remove`); refused with `still-referenced` while
    /// an entity other than a relationship needs it.
    #[wasm_bindgen(js_name = removeWithRelationships)]
    pub fn remove_with_relationships_js(&mut self, id: u64) -> Result<(), JsValue> {
        self.0.remove_with_relationships(id).map_err(js_error)
    }

    /// The handle of the entity operation `index` of an `author` batch
    /// produces, usable wherever a later operation takes an id (`bigint`).
    #[wasm_bindgen(js_name = handle)]
    pub fn handle_js(index: u32) -> u64 {
        openbim_ifc_binding_core::authoring::HANDLE_BASE + u64::from(index)
    }

    /// Every `[from, to]` pair (`bigint`s) where `to` does not exist.
    #[wasm_bindgen(
        js_name = danglingReferences,
        unchecked_return_type = "[bigint, bigint][]"
    )]
    pub fn dangling_references_js(&self) -> Array {
        self.0
            .dangling_references()
            .into_iter()
            .map(|(from, to)| Array::of2(&big(from), &big(to)))
            .collect()
    }
}
