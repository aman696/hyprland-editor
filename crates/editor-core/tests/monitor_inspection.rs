use hyprland_editor_core::{
    lua::{
        monitor_inspection::{self, FindingKind},
        replay::{self, Document, Event, Observed, Profile, Program},
        scalar_codec::{NativeValue, Outcome, Rejection},
        source_hash,
    },
    AUDITED_MAIN, BASELINE,
};
use serde::Deserialize;

fn event(order: usize, arguments: Vec<Observed>) -> Event {
    Event {
        order,
        document: "main".into(),
        span: 0..1,
        api: "hl.monitor".into(),
        arguments,
        argument_sources: vec![],
        path_prefix: 0,
    }
}
fn table(fields: &[(&[u8], Observed)]) -> Observed {
    Observed::Table(
        fields
            .iter()
            .map(|(k, v)| (Observed::Bytes(k.to_vec()), v.clone()))
            .collect(),
    )
}
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "main".into(),
        documents: vec![Document {
            id: "main".into(),
            load_name: b"<capture>/main.lua".to_vec(),
            source: source.into(),
            sha256: source_hash(source),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
#[derive(Deserialize)]
struct Fixtures {
    cases: Vec<Fixture>,
}
#[derive(Deserialize)]
struct Fixture {
    version: String,
    case: String,
    lua: String,
    observation: Observation,
}
#[derive(Deserialize)]
struct Observation {
    arguments: Vec<Observed>,
    delegated: Vec<Delegated>,
    added: Vec<Added>,
    refresh_requests: usize,
}
#[derive(Deserialize)]
struct Delegated {
    key_hex: String,
    input: Observed,
}
#[derive(Deserialize)]
struct Added {
    output_hex: String,
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn monitor_dispatch_matches_pinned_handler_observations_without_claiming_native_application() {
    let fixtures: Fixtures =
        serde_json::from_str(include_str!("../../../data/monitor-source-fixtures.json")).unwrap();
    assert_eq!(fixtures.cases.len(), 86);
    let mut compared = 0;
    for fixture in fixtures.cases {
        let version = if fixture.version == "baseline" {
            BASELINE
        } else {
            AUDITED_MAIN
        };
        // These fixtures intentionally exercise real Lua metatables. The static
        // evaluator must stop before them, rather than model a raw table as complete.
        if matches!(
            fixture.case.as_str(),
            "output_index_lookup" | "pairs_is_ignored"
        ) {
            let report = replay::replay(&program(&fixture.lua, version)).unwrap();
            assert!(report.halt.is_some());
            assert!(report.events.is_empty());
            continue;
        }
        let event = event(0, fixture.observation.arguments);
        let result = monitor_inspection::inspect(std::slice::from_ref(&event), version).unwrap();
        assert!(result.stopped.is_none(), "{}", fixture.case);
        assert!(!result.runtime_verified);
        assert_eq!(result.numeric_locale, "C");
        let batch = &result.batches[0];
        assert!(!batch.existing_rule_state_known);
        assert!(!batch.order_within_call_known);
        assert!(!batch.native_application_verified);
        assert_eq!(
            batch.models_submission_and_refresh,
            fixture.observation.refresh_requests == 1,
            "{}",
            fixture.case
        );
        assert_eq!(
            batch.output_bytes.as_deref().map(hex),
            fixture
                .observation
                .added
                .first()
                .map(|a| a.output_hex.clone()),
            "{}",
            fixture.case
        );
        let mut actual: Vec<_> = batch
            .candidates
            .iter()
            .map(|c| {
                (
                    hex(&c.raw_key),
                    serde_json::to_string(c.value(&event).unwrap()).unwrap(),
                )
            })
            .collect();
        let mut expected: Vec<_> = fixture
            .observation
            .delegated
            .iter()
            .map(|d| (d.key_hex.clone(), serde_json::to_string(&d.input).unwrap()))
            .collect();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{}", fixture.case);
        // Parser/apply refusals in the independent routing fixture are inert
        // service adapters. They are not evidence for primary parser acceptance.
        let replay = replay::replay(&program(&fixture.lua, version)).unwrap();
        assert!(replay.halt.is_none(), "{}: {:?}", fixture.case, replay.halt);
        assert_eq!(replay.events.len(), 1);
        assert_eq!(replay.returns, [Observed::Int(0)]);
        compared += 1;
    }
    assert_eq!(compared, 82);
}

#[test]
fn monitor_primary_codecs_enforce_bounds_preserve_raw_aliases_and_continue_after_bad_fields() {
    for version in [BASELINE, AUDITED_MAIN] {
        assert_eq!(monitor_inspection::fields(version).unwrap().len(), 22);
        let input = event(
            7,
            vec![table(&[
                (b"output", Observed::Bytes(b"DP-1".to_vec())),
                (b"transform", Observed::Int(8)),
                (b"reserved", Observed::Int(-1)),
                (b"reserved_area", Observed::Int(2)),
                (b"mode\0alias", Observed::Bytes(b"preferred".to_vec())),
                (b"mode", Observed::Bytes(b"highrr".to_vec())),
                (b"disabled", Observed::Bool(true)),
                (b"icc", Observed::Bytes(b"/unopened/profile.icc".to_vec())),
                (b"supports_hdr", Observed::Int(-1)),
                (b"sdrbrightness", Observed::Float(1.5)),
            ])],
        );
        let result = monitor_inspection::inspect(&[input], version).unwrap();
        let batch = &result.batches[0];
        let field = |key: &str| batch.candidates.iter().find(|c| c.field == key).unwrap();
        assert!(matches!(
            field("transform").primary_value,
            Outcome::Rejected {
                error: Rejection::OutOfRange
            }
        ));
        assert!(matches!(
            field("reserved").primary_value,
            Outcome::Accepted {
                native: NativeValue::CssGap([-1, -1, -1, -1]),
                ..
            }
        ));
        assert!(
            matches!(field("sdrbrightness").primary_value, Outcome::Accepted {native: NativeValue::FloatBits(bits), ..} if bits == 1.5_f32.to_bits())
        );
        assert!(matches!(
            field("supports_hdr").primary_value,
            Outcome::Accepted {
                native: NativeValue::Int(-1),
                ..
            }
        ));
        assert!(matches!(
            field("disabled").primary_value,
            Outcome::Accepted {
                native: NativeValue::Bool(true),
                ..
            }
        ));
        assert!(batch.models_submission_and_refresh);
        assert_eq!(batch.candidates.iter().filter(|c| c.ambiguous).count(), 4);
        assert_eq!(
            batch
                .findings
                .iter()
                .filter(|f| f.kind == FindingKind::AmbiguousProperty)
                .count(),
            4
        );
        assert!(batch
            .candidates
            .iter()
            .filter(|c| c.field == "mode")
            .all(|c| c.ambiguous));
        // A string's primitive acceptance does not certify mode syntax, an ICC
        // file, integer narrowing, the monitor manager or physical hardware.
        assert!(!batch.native_application_verified);
    }
}

#[test]
fn monitor_inspection_refuses_invalid_context_duplicates_and_unbounded_observations() {
    let valid = || event(1, vec![table(&[(b"output", Observed::Int(1))])]);
    assert!(monitor_inspection::inspect(&[], "unreviewed").is_err());
    let mut wrong = valid();
    wrong.api = "hl.exec_cmd".into();
    assert!(monitor_inspection::inspect(&[wrong], BASELINE).is_err());
    assert!(monitor_inspection::inspect(&vec![valid(); 257], BASELINE).is_err());
    let result = monitor_inspection::inspect(&[valid(), valid()], BASELINE).unwrap();
    assert_eq!(result.batches.len(), 1);
    assert!(result.stopped.unwrap().contains("not ordered"));
    let duplicates = event(
        0,
        vec![table(&[
            (b"output", Observed::Int(1)),
            (b"output", Observed::Int(2)),
        ])],
    );
    let result = monitor_inspection::inspect(&[duplicates], BASELINE).unwrap();
    assert!(!result.batches[0].models_submission_and_refresh);
    assert_eq!(
        result.batches[0].findings[0].kind,
        FindingKind::DuplicateStringKey
    );
    let mut too_large = valid();
    too_large.order = 2;
    too_large.arguments = vec![Observed::Table(vec![
        (Observed::Int(1), Observed::Int(1));
        4097
    ])];
    let result = monitor_inspection::inspect(&[valid(), too_large], BASELINE).unwrap();
    assert_eq!(result.batches.len(), 1);
    assert!(result.stopped.unwrap().contains("4096"));
    let huge = event(2, vec![Observed::Bytes(vec![255; 300_000])]);
    let result = monitor_inspection::inspect(&[valid(), huge], BASELINE).unwrap();
    assert_eq!(result.batches.len(), 1);
    assert!(result.stopped.unwrap().contains("limit"));
}
