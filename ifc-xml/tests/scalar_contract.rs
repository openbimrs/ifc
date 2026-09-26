//! Property test of the scalar contract: every scalar `Value` survives an
//! ifcXML round trip with its kind and value unchanged.
//!
//! The writer puts a scalar in an XML attribute only when the reader's
//! inference would give it back; everything else becomes an element with an
//! explicit `kind`. A generator biased towards the lexically dangerous cases
//! (numeric-looking text, `i<n>` strings, `inf`/`NaN`, exponents, padding,
//! empty strings, XML metacharacters) drives values through every position a
//! scalar can occupy: a plain attribute, a list item and a typed wrapper.
//!
//! The generator is a fixed-seed SplitMix64, so a failure names a seed and
//! reproduces exactly without a property-testing dependency.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_xml::XmlCodec;

const SEEDS: [u64; 4] = [1, 0x5eed, 0x00c0_ffee, 0xdead_beef_cafe];
const VALUES_PER_SEED: usize = 5_000;

/// Deterministic SplitMix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Strings the lexical rules have to get right, verbatim.
const EDGE_TEXT: &[&str] = &[
    "",
    " ",
    "0",
    "-0",
    "+0",
    "42",
    "+42",
    "-42",
    "0.1",
    ".5",
    "5.",
    "1e5",
    "1E-5",
    "1e+5",
    "1e999",
    "-1e999",
    "1e",
    "e5",
    "inf",
    "-inf",
    "+inf",
    "Infinity",
    "NaN",
    "nan",
    "i",
    "i0",
    "i7",
    "I7",
    "i-1",
    "i+1",
    " i7",
    "i7 ",
    "i 7",
    "ii7",
    "i18446744073709551615",
    "i18446744073709551616",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775809",
    "0x10",
    "1_000",
    "1,5",
    "a0",
    "true",
    "unknown",
    "$",
    "*",
    ".T.",
    "#12",
    "\t1",
    "1\n",
    "\r\n",
    "&amp;",
    "<i7/>",
    "\"'",
    "Außenwand",
    "\u{30d3}\u{30eb}",
];

/// Characters combined into generated strings, weighted to numeric shapes.
const ALPHABET: &[char] = &[
    '0', '1', '5', '9', '.', 'e', 'E', '+', '-', 'i', 'I', 'n', 'f', 'N', 'a', 'x', ' ', '\t',
    '\n', '\r', '&', '<', '>', '"', '\'', ';', 'ß', '\u{30d3}',
];

fn text(rng: &mut Rng) -> String {
    if rng.below(3) == 0 {
        return rng.pick(EDGE_TEXT).to_string();
    }
    let len = rng.below(9);
    (0..len).map(|_| *rng.pick(ALPHABET)).collect()
}

fn real(rng: &mut Rng) -> f64 {
    const EDGE: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        1e-5,
        1e15,
        1e16,
        -1e15,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
        12_345.67,
    ];
    loop {
        let candidate = match rng.below(3) {
            0 => *rng.pick(EDGE),
            1 => (rng.next() as i64 as f64) / 1024.0,
            _ => f64::from_bits(rng.next()),
        };
        // Non-finite reals are refused by the writer, tested separately.
        if candidate.is_finite() {
            return candidate;
        }
    }
}

fn integer(rng: &mut Rng) -> i64 {
    match rng.below(3) {
        0 => *rng.pick(&[0, 1, -1, i64::MAX, i64::MIN, 42]),
        1 => rng.next() as i64 % 1000,
        _ => rng.next() as i64,
    }
}

fn scalar(rng: &mut Rng) -> Value {
    match rng.below(12) {
        0 => Value::Null,
        1 => Value::Derived,
        2 => Value::Bool(rng.below(2) == 0),
        3 => Value::LogicalUnknown,
        4 => Value::Integer(integer(rng)),
        5 => Value::Real(real(rng)),
        6 => Value::Binary(text(rng).into()),
        7 => Value::Enum(text(rng).into()),
        8 => Value::Ref(EntityId(match rng.below(3) {
            0 => *rng.pick(&[0, 1, u64::MAX]),
            _ => rng.next() >> rng.below(64),
        })),
        _ => Value::Text(text(rng).into()),
    }
}

/// Place each scalar as a plain attribute, a list item and a typed wrapper.
fn model_for(values: &[Value]) -> Model {
    let mut model = Model::new();
    for (index, value) in values.iter().enumerate() {
        model.insert(
            EntityId(index as u64 + 1),
            Entity::new(
                "IFCSCALARPROBE",
                vec![
                    value.clone(),
                    Value::List(vec![value.clone(), Value::Integer(0)]),
                    Value::Typed {
                        type_name: "IFCLABEL".into(),
                        value: Box::new(value.clone()),
                    },
                ],
            ),
        );
    }
    model
}

/// Equality with reals compared bit for bit (`-0.0` is not `0.0`).
fn identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Real(left), Value::Real(right)) => left.to_bits() == right.to_bits(),
        (Value::List(left), Value::List(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| identical(l, r))
        }
        (
            Value::Typed {
                type_name: left_type,
                value: left,
            },
            Value::Typed {
                type_name: right_type,
                value: right,
            },
        ) => left_type == right_type && identical(left, right),
        (left, right) => left == right,
    }
}

#[test]
fn every_scalar_round_trips_with_its_kind_in_every_position() {
    let codec = XmlCodec::default();
    for seed in SEEDS {
        let mut rng = Rng(seed);
        let values: Vec<Value> = (0..VALUES_PER_SEED).map(|_| scalar(&mut rng)).collect();
        let model = model_for(&values);
        let xml = codec.write_bytes(&model).expect("write");
        let back = codec
            .read_bytes(&xml)
            .unwrap_or_else(|error| panic!("seed {seed:#x}: read failed: {error}"));
        assert_eq!(back.len(), model.len(), "seed {seed:#x}: entity count");
        for (id, entity) in model.iter() {
            let read = &back.get(id).expect("entity survives").attributes;
            let same = read.len() == entity.attributes.len()
                && entity
                    .attributes
                    .iter()
                    .zip(read)
                    .all(|(left, right)| identical(left, right));
            assert!(
                same,
                "seed {seed:#x}, {id}: wrote {:?}\n read back {read:?}",
                entity.attributes
            );
        }
    }
}

/// Every edge string, alone, as each text-carrying kind.
#[test]
fn every_edge_string_round_trips_as_each_text_kind() {
    let codec = XmlCodec::default();
    let values: Vec<Value> = EDGE_TEXT
        .iter()
        .flat_map(|text| {
            [
                Value::Text((*text).into()),
                Value::Enum((*text).into()),
                Value::Binary((*text).into()),
            ]
        })
        .collect();
    let model = model_for(&values);
    let back = codec
        .read_bytes(&codec.write_bytes(&model).expect("write"))
        .expect("read");
    for (id, entity) in model.iter() {
        assert_eq!(
            back.get(id).expect("entity survives").attributes,
            entity.attributes,
            "{id}"
        );
    }
}

/// The contract is not "always use an element": unambiguous scalars stay
/// readable attributes, which is the reason the inference exists.
#[test]
fn unambiguous_scalars_stay_plain_attributes() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCSCALARPROBE",
            vec![
                Value::Text("plain".into()),
                Value::Integer(-7),
                Value::Real(2.5),
                Value::Ref(EntityId(3)),
                Value::Text("inf".into()),
            ],
        ),
    );
    let xml = String::from_utf8(XmlCodec::default().write_bytes(&model).unwrap()).unwrap();
    for expected in [
        r#"a0="plain""#,
        r#"a1="-7""#,
        r#"a2="2.5""#,
        r#"a3="i3""#,
        r#"a4="inf""#,
    ] {
        assert!(xml.contains(expected), "expected {expected} in:\n{xml}");
    }
}
