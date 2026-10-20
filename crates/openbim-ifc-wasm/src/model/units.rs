//! Unit conversion and a model's length unit (#469): `convertUnit`,
//! `formatLength`, `convertUnitValue` and `convertLengthUnit`.

use openbim_ifc_binding_core::{convert_unit, format_length, LengthUnitOptions, ToRecord};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::error::js_error;
use crate::records;

#[wasm_bindgen(typescript_custom_section)]
const IFC_UNIT_TYPES: &'static str = r#"
/** Options of `IfcModel.convertLengthUnit`. */
export interface LengthUnitOptions {
  /** Replace the area and volume units by the square and cube of the new length unit, rescaling their values. Default `true`. */
  areaAndVolume?: boolean;
  /** Remove the replaced units nothing else uses. Default `true`. */
  removeReplacedUnits?: boolean;
  /** Report only; the model is unchanged. Default `false`. */
  dryRun?: boolean;
}

/** Options of `IfcModel.formatLength`. */
export interface LengthFormatOptions {
  /** `decimal` (default), `feet-inches` or `inches`. */
  style?: "decimal" | "feet-inches" | "inches";
  /** Decimal places, or the fraction of an inch to round to (`8` for 1/8"). Default 2 for `decimal`, 8 otherwise. */
  places?: number;
  /** The unit a `decimal` length is written in; default the value's unit. */
  displayUnit?: string;
  /** Write a whole number of feet as `5'` rather than `5' - 0"`. */
  suppressZeroInches?: boolean;
}

/** Values of one attribute of one entity type that were rescaled. */
export interface ConvertedValues {
  entityType: string;
  attribute: string;
  kind: "length" | "area" | "volume" | "inverse-length" | "inverse-area" | "map-scale";
  /** Entities with a value rescaled here. */
  entities: number;
  /** Numbers rescaled (a point has three). */
  values: number;
}

/** Length-like values of one attribute left as they are. */
export interface KeptValues {
  entityType: string;
  attribute: string;
  reason: "explicit-unit" | "map-unit" | "unitless-parameter" | "unit-not-followed";
  values: number;
}

/** What `IfcModel.convertLengthUnit` did, or, for a dry run, would do. */
export interface LengthUnitReport {
  /** The project length unit before, as named (`MILLI METRE`, `foot`). */
  from: string;
  to: string;
  lengthFactor: number;
  /** When the area unit followed the length unit. */
  areaFactor: number | undefined;
  volumeFactor: number | undefined;
  /** The new length unit entity; `undefined` when nothing changes. */
  lengthUnit: bigint | undefined;
  areaUnit: bigint | undefined;
  volumeUnit: bigint | undefined;
  converted: ConvertedValues[];
  kept: KeptValues[];
  entitiesChanged: number;
  valuesConverted: number;
  /** Entities created, or for a dry run the ids the commit will create. */
  created: bigint[];
  /** Entities removed, or for a dry run planned for removal. */
  removed: bigint[];
  /** `false` for a dry run or an unchanged unit. */
  applied: boolean;
}
"#;

/// Field `key` of an options object; `undefined` when absent.
fn field(options: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    if options.is_undefined() || options.is_null() {
        return Ok(JsValue::UNDEFINED);
    }
    if !options.is_object() {
        return Err(js_error(
            openbim_ifc_binding_core::BindingError::InvalidValue(
                "options must be an object".into(),
            ),
        ));
    }
    js_sys::Reflect::get(options, &key.into())
}

/// The boolean `key` of an options object, `default` when absent.
fn flag_or(options: &JsValue, key: &str, default: bool) -> Result<bool, JsValue> {
    let value = field(options, key)?;
    if value.is_undefined() || value.is_null() {
        return Ok(default);
    }
    value.as_bool().ok_or_else(|| {
        js_error(openbim_ifc_binding_core::BindingError::InvalidValue(
            format!("`{key}` must be a boolean"),
        ))
    })
}

#[wasm_bindgen]
impl IfcModel {
    /// Convert `value` from the unit named `from` to the unit named `to`:
    /// an SI unit with its prefix (`METRE`, `MILLIMETRE`, `MILLI METRE`,
    /// `SQUARE_MILLIMETRE`, `KILOGRAM`) or a named unit of IFC4 Table 673
    /// with exact factors (`inch`, `foot`, `square foot`, `gallon US`,
    /// `pound` ...). Throws `type-mismatch` for units of different
    /// dimensions, `invalid-value` for an unknown unit, `unsupported` for a
    /// unit with an offset (degree Fahrenheit, #110).
    #[wasm_bindgen(js_name = convertUnit)]
    pub fn convert_unit_js(value: f64, from: &str, to: &str) -> Result<f64, JsValue> {
        convert_unit(value, from, to).map_err(js_error)
    }

    /// Write `value`, a length in the unit named `unit`, as text: decimal in
    /// `displayUnit`, or feet and inches with fractions (`5' - 6 1/2"`).
    #[wasm_bindgen(js_name = formatLength)]
    pub fn format_length_js(
        value: f64,
        unit: &str,
        #[wasm_bindgen(unchecked_param_type = "LengthFormatOptions | undefined")] options: &JsValue,
    ) -> Result<String, JsValue> {
        let style = field(options, "style")?
            .as_string()
            .unwrap_or_else(|| "decimal".to_owned());
        let places = field(options, "places")?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let places = match places.as_f64() {
            Some(places) if places >= 0.0 && places.fract() == 0.0 => places as u32,
            Some(_) => {
                return Err(js_error(
                    openbim_ifc_binding_core::BindingError::InvalidValue(
                        "`places` must be a whole number".into(),
                    ),
                ))
            }
            None if style == "decimal" => 2,
            None => 8,
        };
        let display = field(options, "displayUnit")?.as_string();
        let suppress = flag_or(options, "suppressZeroInches", false)?;
        format_length(value, unit, &style, places, display.as_deref(), suppress).map_err(js_error)
    }

    /// Convert `value` from unit entity `from` to unit entity `to` of this
    /// model (`IfcSIUnit`, `IfcConversionBasedUnit`, `IfcDerivedUnit`).
    #[wasm_bindgen(js_name = convertUnitValue)]
    pub fn convert_unit_value_js(&self, value: f64, from: u64, to: u64) -> Result<f64, JsValue> {
        self.0.convert_unit_value(value, from, to).map_err(js_error)
    }

    /// Convert the model's length unit to the unit named `target` (an SI
    /// `METRE` with any prefix, or `inch`, `foot`, `yard`, `mile`, `US
    /// survey foot`), rescaling every length, area and volume value as one
    /// checked transaction; with `dryRun`, only report. Values with a unit
    /// of their own keep it; values the conversion cannot carry (a p-curve
    /// off a plane, a polynomial curve, a parameter on an undefined
    /// parameter space) refuse the whole conversion with `unsupported`,
    /// each named.
    #[wasm_bindgen(js_name = convertLengthUnit, unchecked_return_type = "LengthUnitReport")]
    pub fn convert_length_unit_js(
        &mut self,
        target: &str,
        #[wasm_bindgen(unchecked_param_type = "LengthUnitOptions | undefined")] options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut parsed = LengthUnitOptions::new();
        parsed.area_and_volume = flag_or(options, "areaAndVolume", true)?;
        parsed.remove_replaced_units = flag_or(options, "removeReplacedUnits", true)?;
        parsed.dry_run = flag_or(options, "dryRun", false)?;
        let report = self
            .0
            .convert_length_unit(target, parsed)
            .map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }
}
