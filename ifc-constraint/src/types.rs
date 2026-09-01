//! Typed IFC4 constraint enumerations and metric value projection.

use ifc_model::{EntityId, Value};

macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $token:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $(#[doc = concat!("IFC token `", $token, "`.")]
            $variant),+
        }
        impl $name {
            pub(crate) fn parse(value: &str) -> Option<Self> {
                $(if value.eq_ignore_ascii_case($token) { return Some(Self::$variant); })+
                None
            }
            pub(crate) const fn token(self) -> &'static str {
                match self { $(Self::$variant => $token),+ }
            }
        }
    };
}

string_enum!(
    /// Severity/handling grade of an IFC constraint.
    ConstraintGrade {
        Hard => "HARD",
        Soft => "SOFT",
        Advisory => "ADVISORY",
        UserDefined => "USERDEFINED",
        NotDefined => "NOTDEFINED",
    }
);

string_enum!(
    /// Comparison operator carried by `IfcMetric`.
    Benchmark {
        GreaterThan => "GREATERTHAN",
        GreaterThanOrEqualTo => "GREATERTHANOREQUALTO",
        LessThan => "LESSTHAN",
        LessThanOrEqualTo => "LESSTHANOREQUALTO",
        EqualTo => "EQUALTO",
        NotEqualTo => "NOTEQUALTO",
        Includes => "INCLUDES",
        NotIncludes => "NOTINCLUDES",
        IncludedIn => "INCLUDEDIN",
        NotIncludedIn => "NOTINCLUDEDIN",
    }
);

string_enum!(
    /// Logical aggregation operator carried by `IfcObjective`.
    LogicalOperator {
        LogicalAnd => "LOGICALAND",
        LogicalOr => "LOGICALOR",
        LogicalXor => "LOGICALXOR",
        LogicalNotAnd => "LOGICALNOTAND",
        LogicalNotOr => "LOGICALNOTOR",
    }
);

string_enum!(
    /// Purpose qualifier carried by `IfcObjective`.
    ObjectiveQualifier {
        CodeCompliance => "CODECOMPLIANCE",
        CodeWaiver => "CODEWAIVER",
        DesignIntent => "DESIGNINTENT",
        External => "EXTERNAL",
        HealthAndSafety => "HEALTHANDSAFETY",
        MergeConflict => "MERGECONFLICT",
        ModelView => "MODELVIEW",
        Parameter => "PARAMETER",
        Requirement => "REQUIREMENT",
        Specification => "SPECIFICATION",
        TriggerCondition => "TRIGGERCONDITION",
        UserDefined => "USERDEFINED",
        NotDefined => "NOTDEFINED",
    }
);

/// Preserved `IfcMetricValueSelect` without evaluating its meaning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricValue<'m> {
    /// Entity-valued member such as `IfcTable` or `IfcAppliedValue`.
    Entity(EntityId),
    /// Explicitly typed scalar/select member and its untouched payload.
    Typed {
        /// Declared IFC type name.
        type_name: &'m str,
        /// Untouched wrapped value.
        value: &'m Value,
    },
}

/// Authoring form of a preserved `IfcMetricValueSelect`.
#[derive(Debug, Clone, Copy)]
pub enum MetricValueDraft<'a> {
    /// Existing or earlier-staged entity-valued member.
    Entity(EntityId),
    /// Explicit IFC type name and payload to wrap.
    Typed {
        /// Type admitted by `IfcMetricValueSelect`.
        type_name: &'a str,
        /// Structural payload preserved verbatim.
        value: &'a Value,
    },
}
