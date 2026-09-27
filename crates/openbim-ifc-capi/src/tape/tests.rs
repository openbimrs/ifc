use super::*;

fn node(kind: i32) -> OpenbimIfcValueNode {
    OpenbimIfcValueNode {
        kind,
        ..OpenbimIfcValueNode::default()
    }
}

fn every_kind() -> Tagged {
    Tagged::List(vec![
        Tagged::Null,
        Tagged::Derived,
        Tagged::Bool(false),
        Tagged::Unknown,
        Tagged::Integer(i64::MIN),
        Tagged::Real(-0.5),
        Tagged::Text("Wand ä".into()),
        Tagged::Binary("0A1".into()),
        Tagged::Enum("STANDARD".into()),
        Tagged::Ref(42),
        Tagged::List(vec![]),
        Tagged::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Tagged::Text(String::new())),
        },
    ])
}

#[test]
fn every_kind_survives_the_tape() {
    let value = every_kind();
    let tape = Tape::encode(&value);
    let back = Reader::new(&tape.nodes, &tape.strings).single().unwrap();
    assert_eq!(back, value);
}

#[test]
fn the_kinds_a_c_host_could_confuse_stay_distinct() {
    let codes: Vec<i32> = [
        Tagged::Null,
        Tagged::Derived,
        Tagged::Unknown,
        Tagged::Bool(false),
    ]
    .iter()
    .map(|value| Tape::encode(value).nodes[0].kind)
    .collect();
    assert_eq!(
        codes,
        [
            OPENBIM_IFC_KIND_NULL,
            OPENBIM_IFC_KIND_DERIVED,
            OPENBIM_IFC_KIND_UNKNOWN,
            OPENBIM_IFC_KIND_BOOL
        ]
    );
}

#[test]
fn several_values_share_one_string_buffer() {
    let values = vec![Tagged::Text("a".into()), Tagged::Enum("B".into())];
    let tape = Tape::encode_all(&values);
    assert_eq!(tape.strings, b"aB");
    assert_eq!((tape.nodes[1].str_offset, tape.nodes[1].str_len), (1, 1));
    let back = Reader::new(&tape.nodes, &tape.strings).many(2).unwrap();
    assert_eq!(back, values);
}

fn refused(nodes: &[OpenbimIfcValueNode], strings: &[u8]) -> String {
    match Reader::new(nodes, strings).single() {
        Err(BindingError::InvalidValue(detail)) => detail,
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn a_malformed_tape_is_refused_not_guessed() {
    assert!(refused(&[], b"").contains("tape ends"));
    assert!(refused(&[node(99)], b"").contains("unknown kind"));
    assert!(refused(&[node(-1)], b"").contains("unknown kind"));

    let bool_two = OpenbimIfcValueNode {
        int_value: 2,
        ..node(OPENBIM_IFC_KIND_BOOL)
    };
    assert!(refused(&[bool_two], b"").contains("0 or 1"));

    let negative_ref = OpenbimIfcValueNode {
        int_value: -1,
        ..node(OPENBIM_IFC_KIND_REF)
    };
    assert!(refused(&[negative_ref], b"").contains("negative"));

    let text_past_end = OpenbimIfcValueNode {
        str_offset: 1,
        str_len: 5,
        ..node(OPENBIM_IFC_KIND_TEXT)
    };
    assert!(refused(&[text_past_end], b"abc").contains("outside"));

    let overflow = OpenbimIfcValueNode {
        str_offset: u64::MAX,
        str_len: 2,
        ..node(OPENBIM_IFC_KIND_TEXT)
    };
    assert!(refused(&[overflow], b"abc").contains("outside"));

    let bad_utf8 = OpenbimIfcValueNode {
        str_len: 1,
        ..node(OPENBIM_IFC_KIND_TEXT)
    };
    assert!(refused(&[bad_utf8], &[0xFF]).contains("UTF-8"));

    let null_with_string = OpenbimIfcValueNode {
        str_len: 1,
        ..node(OPENBIM_IFC_KIND_NULL)
    };
    assert!(refused(&[null_with_string], b"x").contains("no string"));

    let enum_with_children = OpenbimIfcValueNode {
        child_count: 1,
        ..node(OPENBIM_IFC_KIND_ENUM)
    };
    assert!(refused(&[enum_with_children, node(0)], b"").contains("children"));
}

#[test]
fn counts_must_match_the_tape_exactly() {
    let list_of_two = OpenbimIfcValueNode {
        child_count: 2,
        ..node(OPENBIM_IFC_KIND_LIST)
    };
    // One child short: the list runs off the end.
    assert!(refused(&[list_of_two, node(0)], b"").contains("tape ends"));
    // One node too many: trailing data is refused, not ignored.
    assert!(refused(&[node(0), node(0)], b"").contains("only 1 were used"));
    // A claimed billion children fails on the tape, without allocating them.
    let huge = OpenbimIfcValueNode {
        child_count: u32::MAX,
        ..node(OPENBIM_IFC_KIND_LIST)
    };
    assert!(refused(&[huge], b"").contains("tape ends"));
}

#[test]
fn nesting_is_bounded() {
    let deep: Vec<_> = (0..=MAX_NESTING + 1)
        .map(|_| OpenbimIfcValueNode {
            child_count: 1,
            ..node(OPENBIM_IFC_KIND_LIST)
        })
        .chain([node(0)])
        .collect();
    assert!(refused(&deep, b"").contains("nesting"));
}
