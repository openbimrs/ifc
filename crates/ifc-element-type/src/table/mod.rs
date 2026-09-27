//! The generated element-type catalogue.
//!
//! Source: `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`, via
//! `scripts/gen-element-types.py`. Do not edit by hand.
//!
//! # Two slot layouts, not one
//!
//! Element types put `RepresentationMaps` at 6 and `Tag` at 7.
//! Resource and process types put `Identification` at 6 and
//! `LongDescription` at 7. Nine types use the second layout, so
//! a writer that assumed `Tag` would file a tag as a description
//! on every resource and process type.
//!
//! `PredefinedType` lands at 9 for most types, 10 for
//! `IfcFurnitureType`, and 11 for the six resource types, whose
//! supertype interposes `BaseCosts` and `BaseQuantity`.

/// Which supertype layout an element type follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// `RepresentationMaps` at 6, `Tag` at 7, `ElementType` at 8.
    Element,
    /// `Identification` at 6, `LongDescription` at 7,
    /// `ResourceType` or `ProcessType` at 8.
    ResourceOrProcess,
}

/// One element type: its STEP name, slots, and enum tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementType {
    /// STEP type name, upper-case as stored.
    pub type_name: &'static str,
    /// Total attribute count, including inherited.
    pub arity: usize,
    /// Slot holding `PredefinedType`.
    pub predefined_slot: usize,
    /// Whether `PredefinedType` is itself optional.
    pub predefined_optional: bool,
    /// Name of the attribute `USERDEFINED` falls back to.
    pub fallback_attr: &'static str,
    /// Slot of that fallback attribute. Always 8.
    pub fallback_slot: usize,
    /// Which supertype layout slots 6 and 7 follow.
    pub family: Family,
    /// Permitted `PredefinedType` tokens.
    pub members: &'static [&'static str],
}

mod a_c;
mod d_f;
mod g_k;
mod l_p;
mod q_s;
mod t_z;

pub use a_c::*;
pub use d_f::*;
pub use g_k::*;
pub use l_p::*;
pub use q_s::*;
pub use t_z::*;

/// Every element type in the catalogue.
pub const ALL: &[ElementType] = &[
    IFCACTUATORTYPE,
    IFCAIRTERMINALBOXTYPE,
    IFCAIRTERMINALTYPE,
    IFCAIRTOAIRHEATRECOVERYTYPE,
    IFCALARMTYPE,
    IFCAUDIOVISUALAPPLIANCETYPE,
    IFCBEAMTYPE,
    IFCBEARINGTYPE,
    IFCBOILERTYPE,
    IFCBUILDINGELEMENTPARTTYPE,
    IFCBUILDINGELEMENTPROXYTYPE,
    IFCBURNERTYPE,
    IFCCABLECARRIERFITTINGTYPE,
    IFCCABLECARRIERSEGMENTTYPE,
    IFCCABLEFITTINGTYPE,
    IFCCABLESEGMENTTYPE,
    IFCCAISSONFOUNDATIONTYPE,
    IFCCHILLERTYPE,
    IFCCHIMNEYTYPE,
    IFCCOILTYPE,
    IFCCOLUMNTYPE,
    IFCCOMMUNICATIONSAPPLIANCETYPE,
    IFCCOMPRESSORTYPE,
    IFCCONDENSERTYPE,
    IFCCONSTRUCTIONEQUIPMENTRESOURCETYPE,
    IFCCONSTRUCTIONMATERIALRESOURCETYPE,
    IFCCONSTRUCTIONPRODUCTRESOURCETYPE,
    IFCCONTROLLERTYPE,
    IFCCONVEYORSEGMENTTYPE,
    IFCCOOLEDBEAMTYPE,
    IFCCOOLINGTOWERTYPE,
    IFCCOURSETYPE,
    IFCCOVERINGTYPE,
    IFCCREWRESOURCETYPE,
    IFCCURTAINWALLTYPE,
    IFCDAMPERTYPE,
    IFCDISCRETEACCESSORYTYPE,
    IFCDISTRIBUTIONBOARDTYPE,
    IFCDISTRIBUTIONCHAMBERELEMENTTYPE,
    IFCDOORTYPE,
    IFCDUCTFITTINGTYPE,
    IFCDUCTSEGMENTTYPE,
    IFCDUCTSILENCERTYPE,
    IFCELECTRICAPPLIANCETYPE,
    IFCELECTRICDISTRIBUTIONBOARDTYPE,
    IFCELECTRICFLOWSTORAGEDEVICETYPE,
    IFCELECTRICFLOWTREATMENTDEVICETYPE,
    IFCELECTRICGENERATORTYPE,
    IFCELECTRICMOTORTYPE,
    IFCELECTRICTIMECONTROLTYPE,
    IFCELEMENTASSEMBLYTYPE,
    IFCENGINETYPE,
    IFCEVAPORATIVECOOLERTYPE,
    IFCEVAPORATORTYPE,
    IFCEVENTTYPE,
    IFCFANTYPE,
    IFCFASTENERTYPE,
    IFCFILTERTYPE,
    IFCFIRESUPPRESSIONTERMINALTYPE,
    IFCFLOWINSTRUMENTTYPE,
    IFCFLOWMETERTYPE,
    IFCFOOTINGTYPE,
    IFCFURNITURETYPE,
    IFCGEOGRAPHICELEMENTTYPE,
    IFCHEATEXCHANGERTYPE,
    IFCHUMIDIFIERTYPE,
    IFCIMPACTPROTECTIONDEVICETYPE,
    IFCINTERCEPTORTYPE,
    IFCJUNCTIONBOXTYPE,
    IFCKERBTYPE,
    IFCLABORRESOURCETYPE,
    IFCLAMPTYPE,
    IFCLIGHTFIXTURETYPE,
    IFCLIQUIDTERMINALTYPE,
    IFCMECHANICALFASTENERTYPE,
    IFCMEDICALDEVICETYPE,
    IFCMEMBERTYPE,
    IFCMOBILETELECOMMUNICATIONSAPPLIANCETYPE,
    IFCMOORINGDEVICETYPE,
    IFCMOTORCONNECTIONTYPE,
    IFCNAVIGATIONELEMENTTYPE,
    IFCOUTLETTYPE,
    IFCPAVEMENTTYPE,
    IFCPILETYPE,
    IFCPIPEFITTINGTYPE,
    IFCPIPESEGMENTTYPE,
    IFCPLATETYPE,
    IFCPROCEDURETYPE,
    IFCPROTECTIVEDEVICETRIPPINGUNITTYPE,
    IFCPROTECTIVEDEVICETYPE,
    IFCPUMPTYPE,
    IFCRAILTYPE,
    IFCRAILINGTYPE,
    IFCRAMPFLIGHTTYPE,
    IFCRAMPTYPE,
    IFCREINFORCINGBARTYPE,
    IFCREINFORCINGMESHTYPE,
    IFCROOFTYPE,
    IFCSANITARYTERMINALTYPE,
    IFCSENSORTYPE,
    IFCSHADINGDEVICETYPE,
    IFCSIGNTYPE,
    IFCSIGNALTYPE,
    IFCSLABTYPE,
    IFCSOLARDEVICETYPE,
    IFCSPACEHEATERTYPE,
    IFCSPACETYPE,
    IFCSPATIALZONETYPE,
    IFCSTACKTERMINALTYPE,
    IFCSTAIRFLIGHTTYPE,
    IFCSTAIRTYPE,
    IFCSUBCONTRACTRESOURCETYPE,
    IFCSWITCHINGDEVICETYPE,
    IFCSYSTEMFURNITUREELEMENTTYPE,
    IFCTANKTYPE,
    IFCTASKTYPE,
    IFCTENDONANCHORTYPE,
    IFCTENDONCONDUITTYPE,
    IFCTENDONTYPE,
    IFCTRACKELEMENTTYPE,
    IFCTRANSFORMERTYPE,
    IFCTRANSPORTELEMENTTYPE,
    IFCTUBEBUNDLETYPE,
    IFCUNITARYCONTROLELEMENTTYPE,
    IFCUNITARYEQUIPMENTTYPE,
    IFCVALVETYPE,
    IFCVEHICLETYPE,
    IFCVIBRATIONDAMPERTYPE,
    IFCVIBRATIONISOLATORTYPE,
    IFCWALLTYPE,
    IFCWASTETERMINALTYPE,
    IFCWINDOWTYPE,
];
