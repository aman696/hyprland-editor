use hyprland_editor_core::lua::{
    replay::Observed,
    scalar_codec::{complex::Vec2Bounds, Codec, NativeValue, Outcome, Parser},
};
use serde_json::{json, Value as Json};
fn codec(row: &Json) -> Codec {
    match row["kind"].as_str().unwrap() {
        "Color" => Codec::Color,
        "Gradient" => Codec::Gradient,
        "FontWeight" => Codec::FontWeight,
        "Vec2" => Codec::Vec2 {
            validator: (row["profile"] == "bounded").then_some(Vec2Bounds {
                minimum: [0.; 2],
                maximum: [10.; 2],
            }),
        },
        "CssGap" => Codec::CssGap {
            minimum: (row["profile"] == "bounded").then_some(1),
            maximum: (row["profile"] == "bounded").then_some(10),
        },
        _ => panic!("Unknown fixture kind"),
    }
}
#[test]
fn complex_primary_values_match_original_parser_observations_for_both_snapshots() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../data/complex-scalar-fixtures.json")).unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 1064);
    let mut parser = Parser::new().unwrap();
    let mut sparse = 0;
    for row in rows {
        let input: Observed = serde_json::from_value(row["input"].clone()).unwrap();
        let outcome = parser.decode(&codec(row), &input).unwrap();
        let context = format!(
            "{} {} {} {}",
            row["version"], row["kind"], row["profile"], row["expression"]
        );
        if row["expression"].as_str().unwrap().contains("[3]") {
            assert!(
                matches!(outcome, Outcome::Unresolved { .. }),
                "{context}: {outcome:?}"
            );
            sparse += 1;
            continue;
        }
        match outcome {
            Outcome::Accepted { native, .. } => {
                assert_eq!(row["error_code"], 0, "{context}");
                assert_eq!(row["set_by_user"], true, "{context}");
                let value = match native {
                    NativeValue::Color(v) => json!(v),
                    NativeValue::FontWeight(v) => json!(v),
                    NativeValue::CssGap(v) => json!(v),
                    NativeValue::Vec2Bits(v) => json!(v),
                    NativeValue::Gradient { colors, angle_bits } => {
                        json!({"colors":colors,"angle_bits":angle_bits})
                    }
                    other => panic!("Unexpected primitive {other:?}"),
                };
                assert_eq!(value, row["stored"], "{context}");
            }
            Outcome::Rejected { error } => {
                assert_eq!(row["error_code"], error.native_code(), "{context}");
                assert_eq!(row["set_by_user"], false, "{context}");
            }
            other => panic!("Unexpected result for {context}: {other:?}"),
        }
    }
    assert_eq!(sparse, 6);
}
#[test]
fn undefined_casts_ambiguous_snapshots_and_representation_limits_remain_explicit() {
    use hyprland_editor_core::lua::scalar_codec::Representation;
    let mut parser = Parser::new().unwrap();
    for codec in [
        Codec::FontWeight,
        Codec::CssGap {
            minimum: None,
            maximum: None,
        },
    ] {
        for input in [
            Observed::Float(f64::NAN),
            Observed::Float(f64::INFINITY),
            Observed::Float(1e300),
            Observed::Int(i64::MAX),
        ] {
            assert!(matches!(
                parser.decode(&codec, &input).unwrap(),
                Outcome::Unresolved { .. }
            ));
        }
    }
    for input in [b"rgba(1,2,3,-1)".as_slice(), b"rgba(1,2,3,nan)", b"\xff"] {
        assert!(matches!(
            parser
                .decode(&Codec::Color, &Observed::Bytes(input.into()))
                .unwrap(),
            Outcome::Unresolved { .. }
        ));
    }
    let duplicate = Observed::Table(vec![
        (Observed::Bytes(b"top".to_vec()), Observed::Int(1)),
        (Observed::Bytes(b"top".to_vec()), Observed::Int(2)),
    ]);
    assert!(matches!(
        parser
            .decode(
                &Codec::CssGap {
                    minimum: None,
                    maximum: None
                },
                &duplicate
            )
            .unwrap(),
        Outcome::Unresolved { .. }
    ));
    let colors = Observed::Table(
        (1..=33)
            .map(|i| (Observed::Int(i), Observed::Bytes(b"#123".to_vec())))
            .collect(),
    );
    let gradient = Observed::Table(vec![(Observed::Bytes(b"colors".to_vec()), colors)]);
    let Outcome::Accepted {
        native: NativeValue::Gradient { colors, .. },
        representation: Representation::Unavailable { .. },
    } = parser.decode(&Codec::Gradient, &gradient).unwrap()
    else {
        panic!("Large native gradients must stay intact and visibly unavailable in the bounded control");
    };
    assert_eq!(colors.len(), 33);
    let bad = Codec::Vec2 {
        validator: Some(Vec2Bounds {
            minimum: [f32::NAN, 0.],
            maximum: [1.; 2],
        }),
    };
    assert!(parser.decode(&bad, &Observed::Nil).is_err());
    assert!(matches!(
        parser.decode(&Codec::Bool, &Observed::Bool(true)).unwrap(),
        Outcome::Accepted { .. }
    ));
}
