//! Borrowed annotation and presentable-text projections.

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};
use crate::view::Record;

/// Direction in which a text literal is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPath {
    /// `IfcTextPathEnum.LEFT`: text runs right-to-left.
    Left,
    /// `IfcTextPathEnum.RIGHT`: text runs left-to-right.
    Right,
    /// `IfcTextPathEnum.UP`: text runs bottom-to-top.
    Up,
    /// `IfcTextPathEnum.DOWN`: text runs top-to-bottom.
    Down,
}

impl TextPath {
    pub(crate) const fn as_ifc(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
            Self::Up => "UP",
            Self::Down => "DOWN",
        }
    }

    fn parse(value: &str, id: EntityId) -> StyleResult<Self> {
        match value.to_ascii_uppercase().as_str() {
            "LEFT" => Ok(Self::Left),
            "RIGHT" => Ok(Self::Right),
            "UP" => Ok(Self::Up),
            "DOWN" => Ok(Self::Down),
            _ => Err(StyleError::InvalidValue {
                entity: "IFCTEXTLITERAL".to_owned(),
                id,
                attribute: "Path",
                value: value.to_owned(),
            }),
        }
    }
}

/// Alignment of an `IfcTextLiteralWithExtent` inside its planar extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxAlignment {
    /// `top-left`: anchored to the extent's top-left corner.
    TopLeft,
    /// `top-middle`: centred along the extent's top edge.
    TopMiddle,
    /// `top-right`: anchored to the extent's top-right corner.
    TopRight,
    /// `middle-left`: centred along the extent's left edge.
    MiddleLeft,
    /// `center`: centred in both axes within the extent.
    Center,
    /// `middle-right`: centred along the extent's right edge.
    MiddleRight,
    /// `bottom-left`: anchored to the extent's bottom-left corner.
    BottomLeft,
    /// `bottom-middle`: centred along the extent's bottom edge.
    BottomMiddle,
    /// `bottom-right`: anchored to the extent's bottom-right corner.
    BottomRight,
}

impl BoxAlignment {
    pub(crate) const fn as_ifc(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopMiddle => "top-middle",
            Self::TopRight => "top-right",
            Self::MiddleLeft => "middle-left",
            Self::Center => "center",
            Self::MiddleRight => "middle-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomMiddle => "bottom-middle",
            Self::BottomRight => "bottom-right",
        }
    }

    fn parse(value: &str, record: Record<'_, '_>) -> StyleResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "top-left" => Ok(Self::TopLeft),
            "top-middle" => Ok(Self::TopMiddle),
            "top-right" => Ok(Self::TopRight),
            "middle-left" => Ok(Self::MiddleLeft),
            "center" => Ok(Self::Center),
            "middle-right" => Ok(Self::MiddleRight),
            "bottom-left" => Ok(Self::BottomLeft),
            "bottom-middle" => Ok(Self::BottomMiddle),
            "bottom-right" => Ok(Self::BottomRight),
            _ => Err(StyleError::InvalidValue {
                entity: record.entity.type_name.to_string(),
                id: record.id,
                attribute: "BoxAlignment",
                value: value.to_owned(),
            }),
        }
    }
}

/// The `IfcAnnotationTypeEnum` classifying an `IfcAnnotation`'s purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationType {
    /// `CONTOURLINE`: a line of constant elevation or value.
    ContourLine,
    /// `DIMENSION`: a dimension annotation.
    Dimension,
    /// `ISOBAR`: a line of constant pressure.
    Isobar,
    /// `ISOLUX`: a line of constant illuminance.
    Isolux,
    /// `ISOTHERM`: a line of constant temperature.
    Isotherm,
    /// `LEADER`: a leader line pointing from text to a feature.
    Leader,
    /// `SURVEY`: a survey annotation.
    Survey,
    /// `SYMBOL`: a symbolic annotation.
    Symbol,
    /// `TEXT`: a text annotation.
    Text,
    /// `USERDEFINED`: a case named by `ObjectType`, outside this enumeration.
    UserDefined,
    /// `NOTDEFINED`: the annotation kind is intentionally unspecified.
    NotDefined,
}

impl AnnotationType {
    pub(crate) const fn as_ifc(self) -> &'static str {
        match self {
            Self::ContourLine => "CONTOURLINE",
            Self::Dimension => "DIMENSION",
            Self::Isobar => "ISOBAR",
            Self::Isolux => "ISOLUX",
            Self::Isotherm => "ISOTHERM",
            Self::Leader => "LEADER",
            Self::Survey => "SURVEY",
            Self::Symbol => "SYMBOL",
            Self::Text => "TEXT",
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
        }
    }

    fn parse(value: &str, record: Record<'_, '_>) -> StyleResult<Self> {
        match value.to_ascii_uppercase().as_str() {
            "CONTOURLINE" => Ok(Self::ContourLine),
            "DIMENSION" => Ok(Self::Dimension),
            "ISOBAR" => Ok(Self::Isobar),
            "ISOLUX" => Ok(Self::Isolux),
            "ISOTHERM" => Ok(Self::Isotherm),
            "LEADER" => Ok(Self::Leader),
            "SURVEY" => Ok(Self::Survey),
            "SYMBOL" => Ok(Self::Symbol),
            "TEXT" => Ok(Self::Text),
            "USERDEFINED" => Ok(Self::UserDefined),
            "NOTDEFINED" => Ok(Self::NotDefined),
            _ => Err(StyleError::InvalidValue {
                entity: record.entity.type_name.to_string(),
                id: record.id,
                attribute: "PredefinedType",
                value: value.to_owned(),
            }),
        }
    }
}

/// A schema-aware view of `IfcAnnotation`.
#[derive(Debug, Clone, Copy)]
pub struct Annotation<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> Annotation<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StyleResult<Self> {
        Ok(Self { record })
    }

    /// The entity id of this `IfcAnnotation`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `GlobalId` (IFC GUID) attribute. Mandatory.
    pub fn global_id(&self) -> StyleResult<&'m str> {
        self.record.required_text("GlobalId")
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `Description` attribute, when authored.
    pub fn description(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    /// The `ObjectType` attribute, when authored.
    pub fn object_type(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("ObjectType")
    }

    /// The `OwnerHistory` reference to an `IfcOwnerHistory`, when authored.
    pub fn owner_history(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref("OwnerHistory", "IfcOwnerHistory")
    }

    /// The `ObjectPlacement` reference to an `IfcObjectPlacement`, when authored.
    pub fn object_placement(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("ObjectPlacement", "IfcObjectPlacement")
    }

    /// The `Representation` reference to an `IfcProductRepresentation`, when authored.
    pub fn representation(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("Representation", "IfcProductRepresentation")
    }

    /// IFC4X3's annotation predefined type; older schemas return `None`.
    pub fn predefined_type(&self) -> StyleResult<Option<AnnotationType>> {
        self.record
            .optional_enum("PredefinedType")?
            .map(|value| AnnotationType::parse(value, self.record))
            .transpose()
    }
}

/// A schema-aware view of `IfcTextLiteral` (including its subtype).
#[derive(Debug, Clone, Copy)]
pub struct TextLiteral<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextLiteral<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StyleResult<Self> {
        Ok(Self { record })
    }

    /// The entity id of this `IfcTextLiteral`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Literal` attribute: the text content itself.
    pub fn literal(&self) -> StyleResult<&'m str> {
        self.record.required_text("Literal")
    }

    /// The `Placement` reference to the `IfcPlacement` positioning the text.
    pub fn placement(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Placement", "IfcPlacement")
    }

    /// The `Path` attribute: reading direction of the text.
    pub fn path(&self) -> StyleResult<TextPath> {
        TextPath::parse(self.record.required_enum("Path")?, self.record.id)
    }
}

/// A schema-aware view of `IfcTextLiteralWithExtent`.
#[derive(Debug, Clone, Copy)]
pub struct TextLiteralWithExtent<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextLiteralWithExtent<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StyleResult<Self> {
        Ok(Self { record })
    }

    /// The entity id of this `IfcTextLiteralWithExtent`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Literal` attribute: the text content itself.
    pub fn literal(&self) -> StyleResult<&'m str> {
        self.record.required_text("Literal")
    }

    /// The `Placement` reference to the `IfcPlacement` positioning the text.
    pub fn placement(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Placement", "IfcPlacement")
    }

    /// The `Path` attribute: reading direction of the text.
    pub fn path(&self) -> StyleResult<TextPath> {
        TextPath::parse(self.record.required_enum("Path")?, self.record.id)
    }

    /// The `Extent` reference to the `IfcPlanarExtent` bounding the text.
    pub fn extent(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Extent", "IfcPlanarExtent")
    }

    /// The `BoxAlignment` attribute: how the text is anchored within its extent.
    pub fn box_alignment(&self) -> StyleResult<BoxAlignment> {
        BoxAlignment::parse(self.record.required_text("BoxAlignment")?, self.record)
    }
}

/// A bounded fill region made from one outer and optional inner curves.
#[derive(Debug, Clone, Copy)]
pub struct AnnotationFillArea<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> AnnotationFillArea<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StyleResult<Self> {
        Ok(Self { record })
    }

    /// The entity id of this `IfcAnnotationFillArea`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `OuterBoundary` reference to the enclosing `IfcCurve`. Mandatory.
    pub fn outer_boundary(&self) -> StyleResult<EntityId> {
        self.record.required_ref("OuterBoundary", "IfcCurve")
    }

    /// The `InnerBoundaries` references to `IfcCurve`s cut out of the fill area, if any.
    pub fn inner_boundaries(&self) -> StyleResult<Vec<EntityId>> {
        self.record.optional_refs("InnerBoundaries", "IfcCurve")
    }
}
