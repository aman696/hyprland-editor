use hyprland_editor_core::{
    lua::{replay::*, source_hash},
    AUDITED_MAIN, BASELINE,
};

fn program(source: &str) -> Program {
    Program {
        version: BASELINE.into(),
        profile: Profile::SyntheticEmit,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/fixture/root.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn truth(r: &Report) -> Vec<(DecisionKind, bool)> {
    r.path_decisions
        .iter()
        .map(|d| {
            let DecisionOutcome::Truthiness { truthy } = d.outcome else {
                panic!("Not a truth test")
            };
            (d.kind.clone(), truthy)
        })
        .collect()
}
#[test]
fn branches_and_short_circuits_preserve_executed_tests_and_never_invent_skipped_effects() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("local x=0;if false then emit('never')elseif 0 then x=2 else emit('never')end;local y=false and emit('never');local z=true or emit('never');local a=true and (emit(x) or 4);local b=false or 5;emit(b)");
        p.version = version.into();
        let r = replay(&p).unwrap();
        assert!(r.halt.is_none(), "{:?}", r.halt);
        assert_eq!(
            truth(&r),
            vec![
                (DecisionKind::If, false),
                (DecisionKind::ElseIf, true),
                (DecisionKind::And, false),
                (DecisionKind::Or, true),
                (DecisionKind::And, true),
                (DecisionKind::Or, false),
                (DecisionKind::Or, false)
            ]
        );
        assert_eq!(r.events.len(), 2);
        assert_eq!(r.events[0].arguments, vec![Observed::Int(2)]);
        assert_eq!(r.events[0].path_prefix, 5);
        assert_eq!(r.events[1].arguments, vec![Observed::Int(5)]);
        assert_eq!(r.events[1].path_prefix, 7);
        let write = r.events[0].argument_sources[0].storage_write.unwrap();
        assert_eq!(r.storage_writes[write as usize].path_prefix, 2);
        assert!(!r.runtime_verified);
        let roundtrip: Report = serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();
        assert_eq!(roundtrip.path_decisions, r.path_decisions);
    }
}
#[test]
fn loop_paths_freeze_observations_and_include_false_termination_and_integer_boundaries() {
    let p=program("local n=0;while n<2 do n=n+1;emit(n)end;repeat n=n-1;emit(n)until n==0;for i=2,0,-1 do emit(i)end;for j=2,1 do emit('never')end;for k=9223372036854775807,9223372036854775807 do emit(k)end;emit('after')");
    let r = replay(&p).unwrap();
    assert!(r.halt.is_none(), "{:?}", r.halt);
    assert_eq!(
        r.events.iter().map(|e| e.path_prefix).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 6, 7, 8, 11, 12]
    );
    assert_eq!(r.path_decisions.len(), 12);
    assert_eq!(
        r.path_decisions[..5]
            .iter()
            .map(|d| &d.outcome)
            .collect::<Vec<_>>(),
        vec![
            &DecisionOutcome::Truthiness { truthy: true },
            &DecisionOutcome::Truthiness { truthy: true },
            &DecisionOutcome::Truthiness { truthy: false },
            &DecisionOutcome::Truthiness { truthy: false },
            &DecisionOutcome::Truthiness { truthy: true }
        ]
    );
    assert!(matches!(
        r.path_decisions[8].outcome,
        DecisionOutcome::NumericRange {
            current: -1,
            entered: false,
            ..
        }
    ));
    assert!(matches!(
        r.path_decisions[9].outcome,
        DecisionOutcome::NumericRange {
            current: 2,
            entered: false,
            ..
        }
    ));
    assert!(matches!(
        r.path_decisions[11].outcome,
        DecisionOutcome::IntegerRangeOverflow {
            current: i64::MAX,
            step: 1
        }
    ));
    // Inputs to each decision precede the decision. Observation prefixes cannot
    // acquire tests performed later, including a repeat test after its body.
    for (index, d) in r.path_decisions.iter().enumerate() {
        for input in &d.inputs {
            assert!(r.expression_sources[*input as usize].path_prefix <= index);
        }
        assert!(p.documents[d.site.document]
            .source
            .get(d.site.span.clone())
            .is_some());
    }
}
#[test]
fn history_remains_conservative_across_returns_caught_errors_and_loaded_modules() {
    let mut p=program("local function choose()if true then return 8 end end;local x=choose();pcall(function()if false then return end;error('caught')end);if x==8 then local v=require('part');emit(v)end;emit(x)");
    let module = "if false then emit('never')else return 9 end";
    p.documents.push(Document {
        id: "part".into(),
        source: module.into(),
        sha256: source_hash(module),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let first = replay(&p).unwrap();
    let Some(Halt::NeedsResolution { request }) = first.halt else {
        panic!("Missing module request")
    };
    assert_eq!(first.loads[0].path_prefix, 3);
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let r = replay(&p).unwrap();
    assert!(r.halt.is_none(), "{:?}", r.halt);
    assert_eq!(r.path_decisions.len(), 4);
    assert_eq!(r.path_decisions[3].site.document, 1);
    assert_eq!(r.loads[0].path_prefix, 3);
    assert_eq!(
        r.events.iter().map(|e| e.path_prefix).collect::<Vec<_>>(),
        vec![4, 4]
    );
    assert_eq!(r.events[0].arguments, vec![Observed::Int(9)]);
    assert_eq!(r.events[1].arguments, vec![Observed::Int(8)]);
    // The last observation retains unrelated earlier module decisions too.
    // That overapproximation must not be advertised as a minimal control slice.
    assert_eq!(r.events[1].path_prefix, r.path_decisions.len());
}
#[test]
fn path_history_is_bounded_and_failure_cannot_be_swallowed_by_pcall() {
    let p = program("pcall(function()while true do end end);emit('never')");
    let r = replay(&p).unwrap();
    assert!(matches!(r.halt, Some(Halt::Budget { .. })));
    assert!(r.events.is_empty());
    assert!(!r.path_decisions.is_empty());
    assert!(r.path_decisions.len() <= 8192);
    assert!(serde_json::to_vec(&r).unwrap().len() < 2 * 1024 * 1024);
    assert!(r
        .expression_sources
        .iter()
        .all(|s| s.path_prefix <= r.path_decisions.len()));
    assert!(r
        .storage_writes
        .iter()
        .all(|s| s.path_prefix <= r.path_decisions.len()));
}
