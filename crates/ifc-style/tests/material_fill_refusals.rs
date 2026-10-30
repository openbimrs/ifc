//! `StyleUnits` and the typed refusals of `StyleIndex::material_fill` (#597).

use ifc_style::{StyleError, StyleUnits};

#[test]
fn style_units_refuse_non_positive_factors() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let length = StyleUnits::new(Some(bad), Some(1.0));
        assert!(
            matches!(
                length,
                Err(StyleError::InvalidUnits {
                    factor: "metres_per_length",
                    ..
                })
            ),
            "{bad}: {length:?}"
        );
        let angle = StyleUnits::new(Some(1.0), Some(bad));
        assert!(
            matches!(
                angle,
                Err(StyleError::InvalidUnits {
                    factor: "radians_per_angle",
                    ..
                })
            ),
            "{bad}: {angle:?}"
        );
    }
    let none = StyleUnits::new(None, None).unwrap();
    assert_eq!(none.metres_per_length(), None);
    assert_eq!(none.radians_per_angle(), None);
    assert_eq!(
        StyleUnits::new(Some(1.0), Some(1.0)).unwrap(),
        StyleUnits::SI
    );
}
