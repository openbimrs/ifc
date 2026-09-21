//! The generated occurrence catalogue.
//!
//! Source: `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`, via
//! `scripts/gen-occurrences.py`. Do not edit by hand.
//!
//! # Two rules, and why the second one needs the type catalogue
//!
//! `CorrectPredefinedType` is the familiar one: USERDEFINED
//! without `ObjectType` names nothing.
//!
//! `CorrectTypeAssigned` is stronger and has no analogue on the
//! type side. An occurrence may be typed by at most one type,
//! and that type must be the one class the schema pairs with it:
//! an `IfcPump` takes an `IfcPumpType` and nothing else. The
//! pairing is a fact about the schema, so it is recorded here
//! rather than left to the caller.

/// One occurrence class: its slots, enum, and permitted type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurrence {
    /// STEP type name, upper-case as stored.
    pub type_name: &'static str,
    /// Total attribute count, including inherited.
    pub arity: usize,
    /// Slot holding `PredefinedType`, or `None` when the class has none.
    pub predefined_slot: Option<usize>,
    /// Permitted `PredefinedType` tokens; empty when there is no enum.
    pub members: &'static [&'static str],
    /// The one type class `CorrectTypeAssigned` permits, if any.
    pub type_class: Option<&'static str>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;

/// Every occurrence class in the catalogue.
pub const ALL: &[Occurrence] = &[
    IFCACTUATOR,
    IFCAIRTERMINAL,
    IFCAIRTERMINALBOX,
    IFCAIRTOAIRHEATRECOVERY,
    IFCALARM,
    IFCAUDIOVISUALAPPLIANCE,
    IFCBEAM,
    IFCBEARING,
    IFCBOILER,
    IFCBOREHOLE,
    IFCBUILDINGELEMENTPART,
    IFCBUILDINGELEMENTPROXY,
    IFCBUILTELEMENT,
    IFCBURNER,
    IFCCABLECARRIERFITTING,
    IFCCABLECARRIERSEGMENT,
    IFCCABLEFITTING,
    IFCCABLESEGMENT,
    IFCCAISSONFOUNDATION,
    IFCCHILLER,
    IFCCHIMNEY,
    IFCCIVILELEMENT,
    IFCCOIL,
    IFCCOLUMN,
    IFCCOMMUNICATIONSAPPLIANCE,
    IFCCOMPRESSOR,
    IFCCONDENSER,
    IFCCONTROLLER,
    IFCCONVEYORSEGMENT,
    IFCCOOLEDBEAM,
    IFCCOOLINGTOWER,
    IFCCOURSE,
    IFCCOVERING,
    IFCCURTAINWALL,
    IFCDAMPER,
    IFCDEEPFOUNDATION,
    IFCDISCRETEACCESSORY,
    IFCDISTRIBUTIONBOARD,
    IFCDISTRIBUTIONCHAMBERELEMENT,
    IFCDISTRIBUTIONCONTROLELEMENT,
    IFCDISTRIBUTIONFLOWELEMENT,
    IFCDOOR,
    IFCDUCTFITTING,
    IFCDUCTSEGMENT,
    IFCDUCTSILENCER,
    IFCEARTHWORKSCUT,
    IFCEARTHWORKSELEMENT,
    IFCEARTHWORKSFILL,
    IFCELECTRICAPPLIANCE,
    IFCELECTRICDISTRIBUTIONBOARD,
    IFCELECTRICFLOWSTORAGEDEVICE,
    IFCELECTRICFLOWTREATMENTDEVICE,
    IFCELECTRICGENERATOR,
    IFCELECTRICMOTOR,
    IFCELECTRICTIMECONTROL,
    IFCELEMENTASSEMBLY,
    IFCENGINE,
    IFCEVAPORATIVECOOLER,
    IFCEVAPORATOR,
    IFCFAN,
    IFCFASTENER,
    IFCFILTER,
    IFCFIRESUPPRESSIONTERMINAL,
    IFCFLOWINSTRUMENT,
    IFCFLOWMETER,
    IFCFOOTING,
    IFCFURNISHINGELEMENT,
    IFCFURNITURE,
    IFCGEOGRAPHICELEMENT,
    IFCGEOMODEL,
    IFCGEOSLICE,
    IFCGEOTECHNICALSTRATUM,
    IFCHEATEXCHANGER,
    IFCHUMIDIFIER,
    IFCIMPACTPROTECTIONDEVICE,
    IFCINTERCEPTOR,
    IFCJUNCTIONBOX,
    IFCKERB,
    IFCLAMP,
    IFCLIGHTFIXTURE,
    IFCLIQUIDTERMINAL,
    IFCMECHANICALFASTENER,
    IFCMEDICALDEVICE,
    IFCMEMBER,
    IFCMOBILETELECOMMUNICATIONSAPPLIANCE,
    IFCMOORINGDEVICE,
    IFCMOTORCONNECTION,
    IFCNAVIGATIONELEMENT,
    IFCOPENINGELEMENT,
    IFCOUTLET,
    IFCPAVEMENT,
    IFCPILE,
    IFCPIPEFITTING,
    IFCPIPESEGMENT,
    IFCPLATE,
    IFCPROJECTIONELEMENT,
    IFCPROTECTIVEDEVICE,
    IFCPROTECTIVEDEVICETRIPPINGUNIT,
    IFCPUMP,
    IFCRAIL,
    IFCRAILING,
    IFCRAMP,
    IFCRAMPFLIGHT,
    IFCREINFORCEDSOIL,
    IFCREINFORCINGBAR,
    IFCREINFORCINGMESH,
    IFCROOF,
    IFCSANITARYTERMINAL,
    IFCSENSOR,
    IFCSHADINGDEVICE,
    IFCSIGN,
    IFCSIGNAL,
    IFCSLAB,
    IFCSOLARDEVICE,
    IFCSPACEHEATER,
    IFCSTACKTERMINAL,
    IFCSTAIR,
    IFCSTAIRFLIGHT,
    IFCSURFACEFEATURE,
    IFCSWITCHINGDEVICE,
    IFCSYSTEMFURNITUREELEMENT,
    IFCTANK,
    IFCTENDON,
    IFCTENDONANCHOR,
    IFCTENDONCONDUIT,
    IFCTRACKELEMENT,
    IFCTRANSFORMER,
    IFCTRANSPORTELEMENT,
    IFCTUBEBUNDLE,
    IFCUNITARYCONTROLELEMENT,
    IFCUNITARYEQUIPMENT,
    IFCVALVE,
    IFCVEHICLE,
    IFCVIBRATIONDAMPER,
    IFCVIBRATIONISOLATOR,
    IFCVIRTUALELEMENT,
    IFCVOIDINGFEATURE,
    IFCWALL,
    IFCWALLSTANDARDCASE,
    IFCWASTETERMINAL,
    IFCWINDOW,
];
