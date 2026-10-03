//! The public shape of a profile read in SI units.
//!
//! Every length is in metres and every plane angle in radians. The reader in
//! [`super`] converts each value exactly once, so a caller never applies
//! [`crate::units::UnitScale`] to these numbers again.
//!
//! Optional IFC attributes stay `Option`: the schema gives some of them a
//! default (`IfcLShapeProfileDef.Width` defaults to `Depth`,
//! `IfcAsymmetricIShapeProfileDef.TopFlangeThickness` to the bottom flange),
//! and reporting the default as if it were authored would hide what the file
//! actually says. Each field names its default where the schema has one.

use ifc_model::EntityId;

/// One `IfcProfileDef`, resolved into its family parameters.
///
/// Produced by [`super::describe_profile`]. `lower::profile` builds its exact
/// neutral profile from this value, so the numbers a rule check reads are the
/// numbers the kernel receives.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProfileDescription {
    /// The `IfcProfileDef` entity.
    pub entity: EntityId,
    /// Its concrete IFC type in upper case, e.g. `IFCISHAPEPROFILEDEF`.
    pub type_name: String,
    /// `ProfileName`, often a catalogue designation such as `HEA300`.
    pub name: Option<String>,
    /// `IfcParameterizedProfileDef.Position`, in metres.
    ///
    /// `None` when the file omits it (the identity) and for the families
    /// that have no such attribute (arbitrary, composite, derived).
    pub position: Option<ProfilePosition>,
    /// The family and its parameters.
    pub parameters: ProfileParameters,
}

impl ProfileDescription {
    /// Does this profile bound an area?
    ///
    /// `false` when the profile, or any member or parent it is built from, is
    /// an `IfcArbitraryOpenProfileDef` or an `IfcOpenCrossProfileDef`. An open
    /// curve bounds nothing, so it cannot be the cross section of a swept
    /// solid.
    pub fn bounds_area(&self) -> bool {
        match &self.parameters {
            ProfileParameters::ArbitraryOpen { .. } | ProfileParameters::OpenCross { .. } => false,
            ProfileParameters::Composite { profiles, .. } => {
                profiles.iter().all(ProfileDescription::bounds_area)
            }
            ProfileParameters::Derived { parent, .. }
            | ProfileParameters::Mirrored { parent, .. } => parent.bounds_area(),
            _ => true,
        }
    }
}

/// `IfcParameterizedProfileDef.Position`: where the parameterised section sits
/// in the profile plane.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ProfilePosition {
    /// `Location`, in metres.
    pub origin: [f64; 2],
    /// `RefDirection`, normalised; `[1, 0]` when omitted.
    pub x_axis: [f64; 2],
}

impl ProfilePosition {
    /// The local Y axis: X rotated a quarter turn counter-clockwise
    /// (`IfcOrthogonalComplement`).
    pub fn y_axis(&self) -> [f64; 2] {
        [-self.x_axis[1], self.x_axis[0]]
    }
}

/// `IfcDerivedProfileDef.Operator`: a 2D affine map applied to the parent.
///
/// The axes carry the operator's scale, so they are not necessarily unit
/// length. The translation is in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ProfileOperator {
    /// Image of the parent's X axis.
    pub x_axis: [f64; 2],
    /// Image of the parent's Y axis.
    pub y_axis: [f64; 2],
    /// Translation, in metres.
    pub origin: [f64; 2],
}

/// The parameters of one profile family, in metres and radians.
///
/// Names follow the IFC attributes in snake case. Variants are
/// `#[non_exhaustive]`, so a match must end in `..`; a future attribute can be
/// added without breaking a consumer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ProfileParameters {
    /// `IfcRectangleProfileDef`.
    #[non_exhaustive]
    Rectangle {
        /// `XDim`.
        x_dim: f64,
        /// `YDim`.
        y_dim: f64,
    },
    /// `IfcRoundedRectangleProfileDef`.
    #[non_exhaustive]
    RoundedRectangle {
        /// `XDim`.
        x_dim: f64,
        /// `YDim`.
        y_dim: f64,
        /// `RoundingRadius`.
        rounding_radius: f64,
    },
    /// `IfcRectangleHollowProfileDef`.
    #[non_exhaustive]
    RectangleHollow {
        /// `XDim`.
        x_dim: f64,
        /// `YDim`.
        y_dim: f64,
        /// `WallThickness`.
        wall_thickness: f64,
        /// `InnerFilletRadius`.
        inner_fillet_radius: Option<f64>,
        /// `OuterFilletRadius`.
        outer_fillet_radius: Option<f64>,
    },
    /// `IfcCircleProfileDef`.
    #[non_exhaustive]
    Circle {
        /// `Radius`.
        radius: f64,
    },
    /// `IfcCircleHollowProfileDef`.
    #[non_exhaustive]
    CircleHollow {
        /// `Radius`, to the outside of the wall.
        radius: f64,
        /// `WallThickness`.
        wall_thickness: f64,
    },
    /// `IfcEllipseProfileDef`.
    #[non_exhaustive]
    Ellipse {
        /// `SemiAxis1`, along the position's X axis.
        semi_axis_1: f64,
        /// `SemiAxis2`, along the position's Y axis.
        semi_axis_2: f64,
    },
    /// `IfcIShapeProfileDef`.
    #[non_exhaustive]
    IShape {
        /// `OverallWidth`.
        overall_width: f64,
        /// `OverallDepth`.
        overall_depth: f64,
        /// `WebThickness`.
        web_thickness: f64,
        /// `FlangeThickness`.
        flange_thickness: f64,
        /// `FilletRadius`.
        fillet_radius: Option<f64>,
        /// `FlangeEdgeRadius`.
        flange_edge_radius: Option<f64>,
        /// `FlangeSlope`, in radians.
        flange_slope: Option<f64>,
    },
    /// `IfcAsymmetricIShapeProfileDef`.
    #[non_exhaustive]
    AsymmetricIShape {
        /// `BottomFlangeWidth` (`OverallWidth` in IFC2X3).
        bottom_flange_width: f64,
        /// `OverallDepth`.
        overall_depth: f64,
        /// `WebThickness`.
        web_thickness: f64,
        /// `BottomFlangeThickness` (`FlangeThickness` in IFC2X3).
        bottom_flange_thickness: f64,
        /// `BottomFlangeFilletRadius` (`FilletRadius` in IFC2X3).
        bottom_flange_fillet_radius: Option<f64>,
        /// `TopFlangeWidth`.
        top_flange_width: f64,
        /// `TopFlangeThickness`; defaults to the bottom flange thickness.
        top_flange_thickness: Option<f64>,
        /// `TopFlangeFilletRadius`.
        top_flange_fillet_radius: Option<f64>,
        /// `BottomFlangeEdgeRadius`; not declared in IFC2X3.
        bottom_flange_edge_radius: Option<f64>,
        /// `BottomFlangeSlope`, in radians; not declared in IFC2X3.
        bottom_flange_slope: Option<f64>,
        /// `TopFlangeEdgeRadius`; not declared in IFC2X3.
        top_flange_edge_radius: Option<f64>,
        /// `TopFlangeSlope`, in radians; not declared in IFC2X3.
        top_flange_slope: Option<f64>,
    },
    /// `IfcLShapeProfileDef`.
    #[non_exhaustive]
    LShape {
        /// `Depth`.
        depth: f64,
        /// `Width`; defaults to `Depth` (an equal-leg angle).
        width: Option<f64>,
        /// `Thickness`.
        thickness: f64,
        /// `FilletRadius`.
        fillet_radius: Option<f64>,
        /// `EdgeRadius`.
        edge_radius: Option<f64>,
        /// `LegSlope`, in radians.
        leg_slope: Option<f64>,
    },
    /// `IfcTShapeProfileDef`.
    #[non_exhaustive]
    TShape {
        /// `Depth`.
        depth: f64,
        /// `FlangeWidth`.
        flange_width: f64,
        /// `WebThickness`.
        web_thickness: f64,
        /// `FlangeThickness`.
        flange_thickness: f64,
        /// `FilletRadius`.
        fillet_radius: Option<f64>,
        /// `FlangeEdgeRadius`.
        flange_edge_radius: Option<f64>,
        /// `WebEdgeRadius`.
        web_edge_radius: Option<f64>,
        /// `WebSlope`, in radians.
        web_slope: Option<f64>,
        /// `FlangeSlope`, in radians.
        flange_slope: Option<f64>,
    },
    /// `IfcUShapeProfileDef`.
    #[non_exhaustive]
    UShape {
        /// `Depth`.
        depth: f64,
        /// `FlangeWidth`.
        flange_width: f64,
        /// `WebThickness`.
        web_thickness: f64,
        /// `FlangeThickness`.
        flange_thickness: f64,
        /// `FilletRadius`.
        fillet_radius: Option<f64>,
        /// `EdgeRadius`.
        edge_radius: Option<f64>,
        /// `FlangeSlope`, in radians.
        flange_slope: Option<f64>,
    },
    /// `IfcCShapeProfileDef`.
    #[non_exhaustive]
    CShape {
        /// `Depth`.
        depth: f64,
        /// `Width`.
        width: f64,
        /// `WallThickness`.
        wall_thickness: f64,
        /// `Girth`: the returned lip.
        girth: f64,
        /// `InternalFilletRadius`.
        internal_fillet_radius: Option<f64>,
    },
    /// `IfcZShapeProfileDef`.
    #[non_exhaustive]
    ZShape {
        /// `Depth`.
        depth: f64,
        /// `FlangeWidth`.
        flange_width: f64,
        /// `WebThickness`.
        web_thickness: f64,
        /// `FlangeThickness`.
        flange_thickness: f64,
        /// `FilletRadius`.
        fillet_radius: Option<f64>,
        /// `EdgeRadius`.
        edge_radius: Option<f64>,
    },
    /// `IfcTrapeziumProfileDef`.
    #[non_exhaustive]
    Trapezium {
        /// `BottomXDim`.
        bottom_x_dim: f64,
        /// `TopXDim`.
        top_x_dim: f64,
        /// `YDim`.
        y_dim: f64,
        /// `TopXOffset`; may be negative.
        top_x_offset: f64,
    },
    /// `IfcArbitraryClosedProfileDef`: an area bounded by a curve.
    ///
    /// The boundary is reported by reference; reading its geometry is a curve
    /// question, not a profile one.
    #[non_exhaustive]
    ArbitraryClosed {
        /// `OuterCurve`.
        outer_curve: EntityId,
    },
    /// `IfcArbitraryProfileDefWithVoids`.
    #[non_exhaustive]
    ArbitraryWithVoids {
        /// `OuterCurve`.
        outer_curve: EntityId,
        /// `InnerCurves`, one per void.
        inner_curves: Vec<EntityId>,
    },
    /// `IfcArbitraryOpenProfileDef`: an open curve. Bounds no area.
    #[non_exhaustive]
    ArbitraryOpen {
        /// `Curve`.
        curve: EntityId,
    },
    /// `IfcOpenCrossProfileDef` (IFC4X3): an open chain of straight segments,
    /// each a width and a slope, starting at `offset_point`. Bounds no area.
    ///
    /// The values are the authored ones in SI units; the lowering derives the
    /// chain's vertices from them and documents that construction.
    #[non_exhaustive]
    OpenCross {
        /// `HorizontalWidths`: `true` when each width is the horizontal run
        /// of its segment, `false` when it is the length along the slope.
        horizontal_widths: bool,
        /// `Widths`, in metres, one per segment.
        widths: Vec<f64>,
        /// `Slopes`, in radians, one per segment; positive rises along +X.
        slopes: Vec<f64>,
        /// `Tags`, one per point (one more than there are segments), when
        /// authored. They pair points across consecutive sections of a
        /// sectioned sweep and do not change this profile's shape.
        tags: Option<Vec<String>>,
        /// `OffsetPoint`, in metres. `None` means the chain starts at the
        /// profile origin.
        offset_point: Option<[f64; 2]>,
    },
    /// `IfcCenterLineProfileDef`: a path thickened symmetrically.
    #[non_exhaustive]
    CenterLine {
        /// `Curve`, the centre line.
        curve: EntityId,
        /// `Thickness`: the FULL width across the path.
        thickness: f64,
    },
    /// `IfcCompositeProfileDef`: an ordered set of member profiles.
    #[non_exhaustive]
    Composite {
        /// `Profiles`, in authored order.
        profiles: Vec<ProfileDescription>,
        /// `Label`.
        label: Option<String>,
    },
    /// `IfcDerivedProfileDef`: a parent profile under a 2D operator.
    #[non_exhaustive]
    Derived {
        /// `ParentProfile`.
        parent: Box<ProfileDescription>,
        /// `Operator`, with its translation in metres.
        operator: ProfileOperator,
        /// `Label`.
        label: Option<String>,
    },
    /// `IfcMirroredProfileDef`: the parent mirrored about its local Y axis.
    ///
    /// The schema derives the operator, so the file carries none; the mirror
    /// is implied by the type alone.
    #[non_exhaustive]
    Mirrored {
        /// `ParentProfile`.
        parent: Box<ProfileDescription>,
        /// `Label`.
        label: Option<String>,
    },
}
