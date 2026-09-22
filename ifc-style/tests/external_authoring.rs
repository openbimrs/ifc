//! Authoring externally defined styles, texture coordinate indices,
//! the blob texture, and the text style family.
//!
//! These ten entities declare only two WHERE rules between them, both
//! on `IfcBlobTexture`. The rest of what can go wrong lives in the
//! attribute types: closed lower-case string lists, one-based index
//! lists with a minimum length, and aggregates declared `[1:?]`.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};
use ifc_style::{
    create_blob_texture, create_externally_defined_style, create_indexed_polygonal_texture_map,
    create_surface_style_with_textures, create_text_style, create_text_style_text_model,
    create_texture_coordinate_indices, create_texture_coordinate_indices_with_voids,
    BlobTextureDraft, ExternalStyleKind, SizeValue, TextModelDraft,
};

fn model() -> Model {
    Model::new()
}

fn stub(tx: &mut Transaction) -> ifc_model::EntityId {
    tx.create(Entity::new("IFCINDEXEDPOLYGONALFACE", vec![]))
}

/// The two `IfcBlobTexture` WHERE rules.
///
/// `RasterCodeByteStream` counts bits, so an odd hex-digit payload is
/// a partial byte. `SupportedRasterFormat` is a closed upper-case list.
#[test]
fn blob_texture_enforces_its_two_where_rules() {
    let mut model = model();
    let mut tx = Transaction::new(&model);

    let good = BlobTextureDraft {
        repeat_s: true,
        repeat_t: false,
        raster_format: "PNG",
        raster_code: "89504E47",
        ..BlobTextureDraft::default()
    };
    let id = create_blob_texture(&mut tx, ifc4(), good).expect("blob");

    // Odd digit count: half a byte.
    let odd = BlobTextureDraft {
        raster_code: "89504E4",
        ..good
    };
    create_blob_texture(&mut tx, ifc4(), odd).expect_err("RasterCodeByteStream");

    // Lower case is a different string to the schema.
    let lower = BlobTextureDraft {
        raster_format: "png",
        ..good
    };
    create_blob_texture(&mut tx, ifc4(), lower).expect_err("SupportedRasterFormat");

    let bad = BlobTextureDraft {
        raster_format: "TIFF",
        ..good
    };
    create_blob_texture(&mut tx, ifc4(), bad).expect_err("format outside the list");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("entity");
    assert_eq!(entity.type_name.as_ref(), "IFCBLOBTEXTURE");
    assert_eq!(
        entity.attributes[6],
        Value::Binary("89504E47".into()),
        "RasterCode is binary, not text"
    );
}

/// Texture coordinate indices are one-based and at least three long.
#[test]
fn coordinate_indices_reject_zero_and_short_lists() {
    let mut model = model();
    let mut tx = Transaction::new(&model);
    let face = stub(&mut tx);

    let id =
        create_texture_coordinate_indices(&mut tx, ifc4x3(), &[1, 2, 3], face).expect("indices");

    // LIST [3:?]: two indices is not a face.
    create_texture_coordinate_indices(&mut tx, ifc4x3(), &[1, 2], face)
        .expect_err("fewer than three");

    // IfcPositiveInteger starts at 1; 0 would shift every lookup.
    create_texture_coordinate_indices(&mut tx, ifc4x3(), &[0, 1, 2], face).expect_err("zero index");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("entity");
    assert_eq!(
        entity.attributes[0],
        Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]),
        "indices are integers, not reals"
    );
}

/// UNIQUE is declared on the inner list, so it applies per void.
///
/// A repeated index inside one void collapses two corners of that
/// hole onto one coordinate. The same index appearing in two
/// different voids is legal and must not be refused.
#[test]
fn void_indices_are_unique_within_each_void_only() {
    let model = model();
    let mut tx = Transaction::new(&model);
    let face = stub(&mut tx);

    let shared: &[&[i64]] = &[&[4, 5, 6], &[4, 7, 8]];
    create_texture_coordinate_indices_with_voids(&mut tx, ifc4x3(), &[1, 2, 3], face, shared)
        .expect("index 4 may appear in two different voids");

    let repeated: &[&[i64]] = &[&[4, 5, 4]];
    create_texture_coordinate_indices_with_voids(&mut tx, ifc4x3(), &[1, 2, 3], face, repeated)
        .expect_err("repeat within one void");

    let short: &[&[i64]] = &[&[4, 5]];
    create_texture_coordinate_indices_with_voids(&mut tx, ifc4x3(), &[1, 2, 3], face, short)
        .expect_err("void below three indices");

    create_texture_coordinate_indices_with_voids(&mut tx, ifc4x3(), &[1, 2, 3], face, &[])
        .expect_err("outer list is [1:?]");
}

/// A reference naming nothing identifies no external resource.
#[test]
fn an_external_reference_must_name_something() {
    let mut model = model();
    let mut tx = Transaction::new(&model);

    for kind in [
        ExternalStyleKind::HatchStyle,
        ExternalStyleKind::SurfaceStyle,
        ExternalStyleKind::TextFont,
    ] {
        create_externally_defined_style(&mut tx, ifc4(), kind, None, None, None)
            .expect_err("three nulls resolve for no reader");
        create_externally_defined_style(&mut tx, ifc4(), kind, None, None, Some("  "))
            .expect_err("blank is not a name");
    }

    let id = create_externally_defined_style(
        &mut tx,
        ifc4(),
        ExternalStyleKind::TextFont,
        Some("https://example.invalid/fonts.ttf"),
        None,
        Some("Bodoni"),
    )
    .expect("font");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("entity");
    assert_eq!(entity.type_name.as_ref(), "IFCEXTERNALLYDEFINEDTEXTFONT");
    assert_eq!(entity.attributes.len(), 3, "declared arity");
    assert_eq!(entity.attributes[2], Value::Text("Bodoni".into()));
}

/// Each externally defined kind writes its own type name.
#[test]
fn each_external_kind_writes_its_own_type() {
    let mut model = model();
    let mut tx = Transaction::new(&model);
    let mut ids = Vec::new();
    for kind in [
        ExternalStyleKind::HatchStyle,
        ExternalStyleKind::SurfaceStyle,
        ExternalStyleKind::TextFont,
    ] {
        ids.push(
            create_externally_defined_style(&mut tx, ifc4(), kind, None, Some("ref"), None)
                .expect("staged"),
        );
    }
    tx.commit(&mut model).expect("commit");
    let names: Vec<String> = ids
        .iter()
        .map(|id| model.get(*id).expect("entity").type_name.to_string())
        .collect();
    assert_eq!(
        names,
        vec![
            "IFCEXTERNALLYDEFINEDHATCHSTYLE",
            "IFCEXTERNALLYDEFINEDSURFACESTYLE",
            "IFCEXTERNALLYDEFINEDTEXTFONT",
        ]
    );
}

/// The text model's three word attributes are closed and lower-case.
#[test]
fn text_model_words_are_closed_and_case_sensitive() {
    let mut model = model();
    let mut tx = Transaction::new(&model);

    let id = create_text_style_text_model(
        &mut tx,
        ifc4(),
        TextModelDraft {
            text_align: Some("center"),
            text_decoration: Some("underline"),
            text_transform: Some("uppercase"),
            ..TextModelDraft::default()
        },
    )
    .expect("model");

    // 'Center' is not 'center'.
    create_text_style_text_model(
        &mut tx,
        ifc4(),
        TextModelDraft {
            text_align: Some("Center"),
            ..TextModelDraft::default()
        },
    )
    .expect_err("upper case fails WR1");

    // 'middle' is a plausible word the schema does not list.
    create_text_style_text_model(
        &mut tx,
        ifc4(),
        TextModelDraft {
            text_align: Some("middle"),
            ..TextModelDraft::default()
        },
    )
    .expect_err("outside the closed list");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("entity");
    assert_eq!(entity.attributes.len(), 7, "declared arity");
    assert_eq!(entity.attributes[1], Value::Text("center".into()));
}

/// `IfcSizeSelect` is written typed, and each variant keeps its bound.
#[test]
fn size_select_is_written_typed_and_bounded() {
    let mut model = model();
    let mut tx = Transaction::new(&model);

    let id = create_text_style_text_model(
        &mut tx,
        ifc4(),
        TextModelDraft {
            text_indent: Some(SizeValue::Length(-2.0)),
            line_height: Some(SizeValue::NormalisedRatio(0.5)),
            ..TextModelDraft::default()
        },
    )
    .expect("a length may be negative");

    // A normalised ratio above 1 is out of range.
    create_text_style_text_model(
        &mut tx,
        ifc4(),
        TextModelDraft {
            line_height: Some(SizeValue::NormalisedRatio(1.5)),
            ..TextModelDraft::default()
        },
    )
    .expect_err("outside 0 to 1");

    // A positive length may not be zero.
    create_text_style_text_model(
        &mut tx,
        ifc4(),
        TextModelDraft {
            letter_spacing: Some(SizeValue::PositiveLength(0.0)),
            ..TextModelDraft::default()
        },
    )
    .expect_err("zero is not positive");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("entity");
    assert_eq!(
        entity.attributes[0],
        Value::Typed {
            type_name: "IFCLENGTHMEASURE".into(),
            value: Box::new(Value::Real(-2.0)),
        },
        "the select is written with its resolved type"
    );
}

/// Every `[1:?]` aggregate in this family refuses empty.
#[test]
fn required_aggregates_refuse_empty() {
    let mut model = model();
    let mut tx = Transaction::new(&model);
    let texture = stub(&mut tx);
    let mesh = stub(&mut tx);
    let coords = stub(&mut tx);
    let indices = stub(&mut tx);

    create_surface_style_with_textures(&mut tx, ifc4x3(), &[])
        .expect_err("a style with no textures is not a style");
    let style =
        create_surface_style_with_textures(&mut tx, ifc4x3(), &[texture]).expect("textures");

    create_indexed_polygonal_texture_map(&mut tx, ifc4x3(), &[], mesh, coords, &[indices])
        .expect_err("Maps is [1:?]");
    create_indexed_polygonal_texture_map(&mut tx, ifc4x3(), &[texture], mesh, coords, &[])
        .expect_err("TexCoordIndices is [1:?]");
    let map = create_indexed_polygonal_texture_map(
        &mut tx,
        ifc4x3(),
        &[texture],
        mesh,
        coords,
        &[indices],
    )
    .expect("map");

    tx.commit(&mut model).expect("commit");
    assert_eq!(model.get(style).expect("style").attributes.len(), 1);
    assert_eq!(model.get(map).expect("map").attributes.len(), 4);
}

/// `IfcTextStyle.TextFontStyle` is required, the rest optional.
#[test]
fn text_style_requires_its_font() {
    let mut model = model();
    let mut tx = Transaction::new(&model);
    let font = stub(&mut tx);

    create_text_style(&mut tx, ifc4(), Some(" "), None, None, font, None).expect_err("blank name");
    let id = create_text_style(
        &mut tx,
        ifc4(),
        Some("Caption"),
        None,
        None,
        font,
        Some(true),
    )
    .expect("style");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("entity");
    assert_eq!(entity.attributes.len(), 5, "declared arity");
    assert_eq!(entity.attributes[3], Value::Ref(font), "TextFontStyle");
    assert_eq!(entity.attributes[4], Value::Bool(true));
}

/// The hardcoded arities match the schema that declares each entity.
///
/// Three of these ten were added in IFC4X3 and do not exist in IFC4:
/// the indexed polygonal texture map and the two coordinate-index
/// entities. Authoring them against IFC4 is refused, not silently
/// written, so the split is asserted here rather than assumed.
#[test]
fn declared_arities_match_the_schema() {
    for (name, arity) in [
        ("IfcExternallyDefinedHatchStyle", 3),
        ("IfcExternallyDefinedSurfaceStyle", 3),
        ("IfcExternallyDefinedTextFont", 3),
        ("IfcBlobTexture", 7),
        ("IfcSurfaceStyleWithTextures", 1),
        ("IfcTextStyle", 5),
        ("IfcTextStyleTextModel", 7),
    ] {
        for schema in [ifc4(), ifc4x3()] {
            assert_eq!(
                schema.attributes(name).len(),
                arity,
                "{name} arity drifted in {}",
                schema.name()
            );
        }
    }

    for (name, arity) in [
        ("IfcIndexedPolygonalTextureMap", 4),
        ("IfcTextureCoordinateIndices", 2),
        ("IfcTextureCoordinateIndicesWithVoids", 3),
    ] {
        assert_eq!(ifc4x3().attributes(name).len(), arity, "{name} in IFC4X3");
        assert!(
            ifc4().attributes(name).is_empty(),
            "{name} is an IFC4X3 addition and must not resolve in IFC4"
        );
    }
}

/// A payload of the right length but the wrong alphabet is refused.
///
/// Even-length keeps it past `RasterCodeByteStream`, so only the
/// hex check can reject it. Without this the byte-count rule masks
/// the alphabet rule and the second is never exercised.
#[test]
fn a_non_hex_raster_payload_is_refused() {
    let model = model();
    let mut tx = Transaction::new(&model);
    let draft = BlobTextureDraft {
        raster_format: "PNG",
        raster_code: "ZZZZ",
        ..BlobTextureDraft::default()
    };
    create_blob_texture(&mut tx, ifc4(), draft).expect_err("not hex");
    assert!(
        tx.is_empty(),
        "nothing is staged when the payload is refused"
    );
}
