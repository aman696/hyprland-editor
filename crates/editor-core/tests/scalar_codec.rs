use hyprland_editor_core::{
    lua::{
        replay::Observed,
        scalar_codec::{Codec, NamedInteger, NativeValue, Outcome, Parser, Representation},
    },
    Value,
};
use serde_json::Value as Json;
fn unhex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn input(row: &Json) -> Observed {
    let input = &row["input"];
    match input["kind"].as_str().unwrap() {
        "nil" => Observed::Nil,
        "boolean" => Observed::Bool(input["boolean"].as_bool().unwrap()),
        "number" if input["integer"].is_string() => {
            Observed::Int(input["integer"].as_str().unwrap().parse().unwrap())
        }
        "number" => Observed::Float(f64::from_bits(
            u64::from_str_radix(input["float_bits"].as_str().unwrap(), 16).unwrap(),
        )),
        "string" => Observed::Bytes(unhex(input["bytes_hex"].as_str().unwrap())),
        "table" => Observed::Table(vec![]),
        "function" => Observed::Function("fixture-function".into()),
        other => panic!("Unknown native fixture input {other}"),
    }
}
#[test]
fn primitive_models_match_all_independent_native_inputs_and_preserve_rejection_state() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../data/scalar-value-fixtures.json")).unwrap();
    assert_eq!(fixture["format"], "hye-scalar-value-fixtures-1");
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 592);
    let mut parser = Parser::new().unwrap();
    for row in rows {
        let bounded = row["profile"] != "unbounded";
        let codec = match row["kind"].as_str().unwrap() {
            "Bool" => Codec::Bool,
            "Int" => Codec::Int {
                minimum: bounded.then_some(0),
                maximum: bounded.then_some(10),
                names: (row["profile"] == "mapped").then(|| {
                    [("enabled", 1), ("disabled", 0), ("beyond", 11), ("3.0", 7)]
                        .into_iter()
                        .map(|(name, value)| NamedInteger {
                            name: name.as_bytes().into(),
                            value,
                        })
                        .collect()
                }),
            },
            "Float" => Codec::Float {
                minimum: bounded.then_some(0.0),
                maximum: bounded.then_some(if row["profile"] == "precision" {
                    16777216.0
                } else {
                    10.0
                }),
            },
            "String" => Codec::String { validator: None },
            _ => panic!("Unknown native fixture kind"),
        };
        let outcome = parser.decode(&codec, &input(row)).unwrap();
        let context = format!(
            "{} {} {} {}",
            row["version"], row["kind"], row["profile"], row["expression"]
        );
        match outcome {
            Outcome::Accepted {
                native,
                representation,
            } => {
                assert_eq!(row["error_code"], 0, "{context}");
                assert_eq!(row["set_by_user"], true, "{context}");
                let actual = match &native {
                    NativeValue::Bool(v) => serde_json::json!(v),
                    NativeValue::Int(v) => serde_json::json!(v),
                    NativeValue::FloatBits(v) => serde_json::json!(format!("{v:08x}")),
                    NativeValue::Bytes(v) => {
                        serde_json::json!(v.iter().map(|b| format!("{b:02x}")).collect::<String>())
                    }
                };
                assert_eq!(actual, row["stored"], "{context}");
                if let NativeValue::FloatBits(bits) = native {
                    assert_eq!(
                        matches!(representation, Representation::Unavailable { .. }),
                        !f32::from_bits(bits).is_finite(),
                        "{context}"
                    );
                }
                serde_json::to_vec(&representation).unwrap();
            }
            Outcome::Rejected { error } => {
                assert_eq!(row["error_code"], error.native_code(), "{context}");
                assert_eq!(row["set_by_user"], false, "{context}");
                assert_eq!(row["stored"], row["initial"], "{context}");
            }
            Outcome::Unresolved { reason } => {
                panic!("Unexpected unresolved primitive {context}: {reason}")
            }
        }
    }
}
#[test]
fn native_storage_representation_validator_unknowns_and_conversion_limits_stay_distinct() {
    for bits in [
        0x7ff8000000000000,
        0xfff8000000000000,
        0x7ff0000000000000,
        0xfff0000000000000,
        0,
        0x8000000000000000,
        1,
    ] {
        let original = Observed::Float(f64::from_bits(bits));
        let bytes = serde_json::to_vec(&original).unwrap();
        let Observed::Float(value) = serde_json::from_slice(&bytes).unwrap() else {
            panic!("Lost floating observation")
        };
        assert_eq!(value.to_bits(), bits);
    }
    for invalid in [
        serde_json::json!({"type":"float","value":null}),
        serde_json::json!({"type":"float","value":{"bits":"7ff0000000000000","extra":1}}),
        serde_json::json!({"type":"float","value":{"bits":"fff"}}),
    ] {
        assert!(serde_json::from_value::<Observed>(invalid).is_err());
    }
    let mut parser = Parser::new().unwrap();
    let text = Codec::String { validator: None };
    for bytes in [vec![0xff], vec![b'x'; 16385]] {
        assert!(matches!(
            parser.decode(&text, &Observed::Bytes(bytes)).unwrap(),
            Outcome::Accepted {
                representation: Representation::Unavailable { .. },
                ..
            }
        ));
    }
    let result = parser
        .decode(&text, &Observed::Bytes(b"visible\0hidden".to_vec()))
        .unwrap();
    assert!(
        matches!(result, Outcome::Accepted {native: NativeValue::Bytes(ref b), representation: Representation::Value {value: Value::String(ref s)}} if b == b"visible" && s == "visible")
    );
    let validated = Codec::String {
        validator: Some("native-custom-validator".into()),
    };
    assert!(matches!(
        parser
            .decode(&validated, &Observed::Bytes(b"value".to_vec()))
            .unwrap(),
        Outcome::Unresolved { .. }
    ));
    assert!(matches!(
        parser.decode(&validated, &Observed::Nil).unwrap(),
        Outcome::Rejected { .. }
    ));
    assert!(parser
        .decode(&text, &Observed::Bytes(vec![b'x'; 65537]))
        .is_err());
    assert!(
        serde_json::to_value(Codec::Float {
            minimum: Some(f32::NAN),
            maximum: None
        })
        .is_err(),
        "NaN bounds must not silently serialize as absent bounds"
    );
    // A rejected oversized input neither poisons nor leaks state into reuse.
    assert!(matches!(
        parser.decode(&Codec::Bool, &Observed::Bool(false)).unwrap(),
        Outcome::Accepted {
            native: NativeValue::Bool(false),
            ..
        }
    ));
    assert!(parser
        .decode(
            &Codec::Float {
                minimum: Some(f32::NAN),
                maximum: None
            },
            &Observed::Int(0)
        )
        .is_err());
    let invalid_map = Codec::Int {
        minimum: None,
        maximum: None,
        names: Some(vec![NamedInteger {
            name: b"key\0hidden".to_vec(),
            value: 1,
        }]),
    };
    assert!(parser.decode(&invalid_map, &Observed::Int(0)).is_err());
    // C++ integer/FLOAT comparison rounds in binary32; double/FLOAT does not.
    let bound = Codec::Float {
        minimum: None,
        maximum: Some(16777216.0),
    };
    assert!(matches!(
        parser.decode(&bound, &Observed::Int(16777217)).unwrap(),
        Outcome::Accepted {
            native: NativeValue::FloatBits(0x4b800000),
            ..
        }
    ));
    assert!(matches!(
        parser.decode(&bound, &Observed::Float(16777217.0)).unwrap(),
        Outcome::Rejected { .. }
    ));
}
