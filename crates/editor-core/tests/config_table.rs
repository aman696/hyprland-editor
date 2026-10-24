use hyprland_editor_core::{
    catalogue,
    lua::{
        config_table::{self, FindingKind},
        replay::{Event, Observed, ValueSource},
    },
    AUDITED_MAIN, BASELINE,
};
use serde_json::{json, Value};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn signature(value: &Observed) -> (&'static str, String) {
    match value {
        Observed::Nil => ("nil", String::new()),
        Observed::Bool(v) => ("boolean", v.to_string()),
        Observed::Int(v) => ("integer", v.to_string()),
        Observed::Float(v) => ("float", format!("{:016x}", v.to_bits())),
        Observed::Bytes(v) => ("string", hex(v)),
        Observed::Table(_) | Observed::Cycle(_) => ("table", String::new()),
        Observed::Function(_) => ("function", String::new()),
    }
}
#[test]
fn registry_dispatch_matches_exact_pinned_handler_observations_without_certifying_values() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../data/config-table-fixtures.json")).unwrap();
    assert_eq!(fixture["format"], "hye-config-table-fixtures-1");
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 52);
    let catalogue = catalogue();
    for case in fixture["cases"].as_array().unwrap() {
        let version = if case["version"] == "baseline" {
            BASELINE
        } else {
            AUDITED_MAIN
        };
        let event = Event {
            path_prefix: 0,
            order: 7,
            document: "fixture".into(),
            span: 0..1,
            api: "hl.config".into(),
            arguments: serde_json::from_value(case["arguments"].clone()).unwrap(),
            argument_sources: vec![],
        };
        let batch = config_table::inspect(&event, version, &catalogue).unwrap();
        let mut actual = batch
            .candidates
            .iter()
            .map(|c| {
                let (kind, data) = signature(c.value(&event).unwrap());
                json!([hex(c.lua_key.as_bytes()), kind, data])
            })
            .collect::<Vec<_>>();
        actual.sort_by_key(Value::to_string);
        let mut expected = case["delegations"].as_array().unwrap().clone();
        expected.sort_by_key(Value::to_string);
        assert_eq!(actual, expected, "{} {}", case["version"], case["name"]);
        assert!(!batch.native_values_verified);
        assert!(!batch.order_within_call_known);
        assert!(batch.traversal_resolved);
        assert!(batch.candidates.iter().all(|c| c.expression.is_none()));
        if case["name"].as_str().unwrap().contains("collision") {
            assert_eq!(batch.candidates.len(), 2);
            assert!(batch.candidates.iter().all(|c| c.ambiguous));
            assert_eq!(
                batch
                    .findings
                    .iter()
                    .filter(|f| f.kind == FindingKind::AmbiguousTarget)
                    .count(),
                1
            );
        }
        let mut errors = batch
            .findings
            .iter()
            .filter_map(|f| match f.kind {
                FindingKind::InvalidArgument => Some(hex(b"hl.config: argument must be a table")),
                FindingKind::UnknownKey => Some(hex(format!(
                    "fixture: unknown config key '{}'",
                    String::from_utf8_lossy(&f.api_key)
                )
                .as_bytes())),
                _ => None,
            })
            .collect::<Vec<_>>();
        if case["inert_parser_failure"] == true {
            // The observer's deliberate failure is not a production value codec.
            errors.push(hex(
                b"fixture: error setting 'general.border_size': fixture rejection",
            ));
            assert_eq!(case["refresh_requests"], 1);
            assert_eq!(batch.candidates.len(), 2);
        }
        errors.sort();
        assert_eq!(json!(errors), case["errors_hex"], "{}", case["name"]);
        assert_eq!(case["pairs_calls"], 0);
        if case["name"] == "nul_key_alias" {
            assert!(batch.candidates[0].raw_keys[0].contains(&0));
            assert_eq!(batch.candidates[0].lua_key, "general.border_size");
        }
    }
}
#[test]
fn traversal_cycles_and_path_budgets_remain_unresolved_and_value_origins_remain_only_evidence() {
    let catalogue = catalogue();
    let mut event = Event {
        path_prefix: 0,
        order: 1,
        document: "fixture".into(),
        span: 0..1,
        api: "hl.config".into(),
        arguments: vec![Observed::Table(vec![(
            Observed::Bytes(b"outside".to_vec()),
            Observed::Cycle(0),
        )])],
        argument_sources: vec![],
    };
    let batch = config_table::inspect(&event, BASELINE, &catalogue).unwrap();
    assert!(!batch.traversal_resolved);
    assert_eq!(batch.findings[0].kind, FindingKind::UnresolvedCycle);
    event.arguments = vec![Observed::Table(vec![(
        Observed::Bytes(b"general.border_size\0suffix".to_vec()),
        Observed::Int(9),
    )])];
    event.argument_sources = vec![ValueSource {
        expression: Some(10),
        rows: vec![ValueSource {
            expression: Some(11),
            rows: vec![],
            ..Default::default()
        }],
        ..Default::default()
    }];
    let batch = config_table::inspect(&event, BASELINE, &catalogue).unwrap();
    assert_eq!(batch.candidates[0].expression, Some(11));
    assert!(!batch.native_values_verified);
    assert_eq!(batch.candidates[0].value(&event), Some(&Observed::Int(9)));
    event.order += 1;
    assert!(batch.candidates[0].value(&event).is_none());
    event.arguments = vec![Observed::Table(vec![(
        Observed::Bytes(vec![b'x'; 4097]),
        Observed::Int(1),
    )])];
    assert!(config_table::inspect(&event, BASELINE, &catalogue).is_err());
    let mut nested = Observed::Int(1);
    for _ in 0..34 {
        nested = Observed::Table(vec![(Observed::Bytes(b"a".to_vec()), nested)]);
    }
    event.arguments = vec![nested];
    assert!(config_table::inspect(&event, BASELINE, &catalogue).is_err());
    event.api = "emit".into();
    assert!(config_table::inspect(&event, BASELINE, &catalogue).is_err());
}
