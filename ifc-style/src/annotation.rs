//! Borrowed annotation and presentable-text projections.

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};
use crate::view::Record;

/// Direction in which a text literal is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPath {
    Left,
    Right,
    Up,
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
    TopLeft,
    TopMiddle,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomMiddle,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationType {
    ContourLine,
    Dimension,
    Isobar,
    Isolux,
    Isotherm,
    Leader,
    Survey,
    Symbol,
    Text,
    UserDefined,
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

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn global_id(&self) -> StyleResult<&'m str> {
        self.record.required_text("GlobalId")
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn description(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    pub fn object_type(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("ObjectType")
    }

    pub fn owner_history(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref("OwnerHistory", "IfcOwnerHistory")
    }

    pub fn object_placement(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("ObjectPlacement", "IfcObjectPlacement")
    }

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

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn literal(&self) -> StyleResult<&'m str> {
        self.record.required_text("Literal")
    }

    pub fn placement(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Placement", "IfcPlacement")
    }

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

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn literal(&self) -> StyleResult<&'m str> {
        self.record.required_text("Literal")
    }

    pub fn placement(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Placement", "IfcPlacement")
    }

    pub fn path(&self) -> StyleResult<TextPath> {
        TextPath::parse(self.record.required_enum("Path")?, self.record.id)
    }

    pub fn extent(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Extent", "IfcPlanarExtent")
    }

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

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn outer_boundary(&self) -> StyleResult<EntityId> {
        self.record.required_ref("OuterBoundary", "IfcCurve")
    }

    pub fn inner_boundaries(&self) -> StyleResult<Vec<EntityId>> {
        self.record.optional_refs("InnerBoundaries", "IfcCurve")
    }
}
