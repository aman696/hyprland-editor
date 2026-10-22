use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn request(request: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hyprland-editor"))
        .arg("--lua-source-worker")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let data = serde_json::to_vec(&request).unwrap();
    let writer = std::thread::spawn(move || input.write_all(&data));
    let output = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut data = vec![];
        output
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .unwrap();
        data
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Parser worker failed to respect its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let _ = writer.join().unwrap();
    let data = reader.join().unwrap();
    assert!(status.success());
    serde_json::from_slice(&data).unwrap()
}
#[test]
fn monitor_source_inspection_and_comparison_run_in_the_restricted_worker() {
    use hyprland_editor_core::{
        lua::replay::{
            monitor_edit::{Change, Selection},
            Observed,
        },
        AUDITED_MAIN, BASELINE,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=replay_program("local turn=2;hl.monitor{output='DP-1',transform=turn};hl.config{general={border_size=1}}");
        p.version = version.into();
        let reply = request(json!({"ReplayModules":p}));
        let events = &reply["Replayed"]["events"];
        assert!(reply["Replayed"]["halt"].is_null(), "{reply}");
        assert_eq!(events.as_array().unwrap().len(), 2);
        let event: hyprland_editor_core::lua::replay::Event =
            serde_json::from_value(events[0].clone()).unwrap();
        let result = request(json!({"InspectMonitors":{"events":[event],"version":version}}));
        assert_eq!(
            result["Monitors"]["batches"][0]["candidates"][0]["field"],
            "transform"
        );
        assert_eq!(result["Monitors"]["runtime_verified"], false);
        assert_eq!(
            result["Monitors"]["batches"][0]["native_application_verified"],
            false
        );
        let Observed::Table(rows) = &event.arguments[0] else {
            panic!()
        };
        let row = rows
            .iter()
            .position(|(k, _)| matches!(k,Observed::Bytes(v) if v == b"transform"))
            .unwrap();
        let change = Change {
            event_order: event.order,
            row,
            field: "transform".into(),
            selection: Selection::OriginalLiteral,
            value: hyprland_editor_core::Value::Int(3),
        };
        let result = request(json!({"CompareMonitorSource":{"program":p,"change":change}}));
        assert_eq!(
            result["ScalarSourceComparison"]["review"]["modeled_observations_match"], true,
            "{result}"
        );
        assert_eq!(
            result["ScalarSourceComparison"]["plan"]["edits"][0]["replacement"],
            "3"
        );
    }
}
#[test]
fn monitor_gap_layout_comparisons_cross_the_worker_with_original_spelling_and_comments() {
    use hyprland_editor_core::{
        lua::replay::monitor_edit::{Change, Selection},
        Value, AUDITED_MAIN, BASELINE,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        for (source, selection) in [
            ("hl.monitor{output='DP-1',reserved={top=-1 --[[memo]],right='2',bottom=3,left=4, note='stay'}}", Selection::PreserveTableLayout),
            ("local gap={top=-1 --[[memo]],right='2',bottom=3,left=4, note='stay'};hl.monitor{output='DP-1',reserved=gap}", Selection::OriginalTableLayout),
        ] {
            let mut p=replay_program(source);
            p.version=version.into();
            let change=Change {event_order:0,row:1,field:"reserved".into(),selection,value:Value::CssGap([-8,2,3,4])};
            let result=request(json!({"CompareMonitorSource":{"program":p,"change":change}}));
            assert_eq!(result["ScalarSourceComparison"]["review"]["modeled_observations_match"],true,"{result}");
            assert_eq!(result["ScalarSourceComparison"]["table_layout_preserved"],true);
            assert_eq!(result["ScalarSourceComparison"]["review"]["revisions"][0]["content"],source.replacen("top=-1","top=-8",1));
            assert_eq!(result["ScalarSourceComparison"]["review"]["runtime_verified"],false);
        }
    }
}
#[test]
fn monitor_field_presence_edits_cross_the_restricted_worker_with_exact_row_guards() {
    use hyprland_editor_core::{
        lua::replay::{
            monitor_edit::{FieldChange, TableSelection},
            Observed,
        },
        AUDITED_MAIN, BASELINE,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = replay_program("hl.monitor{output='DP-1',transform= -- keep\n 2}");
        p.version = version.into();
        let result = request(
            json!({"CompareMonitorFieldSource":{"program":p,"change":FieldChange::Add {event_order:0,field:"scale".into(),value:hyprland_editor_core::Value::String("auto".into()),selection:TableSelection::SelectedConstructor}}}),
        );
        assert_eq!(
            result["ScalarSourceComparison"]["review"]["modeled_observations_match"], true,
            "{result}"
        );
        let replay = request(json!({"ReplayModules":p}));
        let event: hyprland_editor_core::lua::replay::Event =
            serde_json::from_value(replay["Replayed"]["events"][0].clone()).unwrap();
        let Observed::Table(rows) = &event.arguments[0] else {
            panic!()
        };
        let row = rows
            .iter()
            .position(|(k, _)| matches!(k,Observed::Bytes(b) if b==b"transform"))
            .unwrap();
        let result = request(
            json!({"CompareMonitorFieldSource":{"program":p,"change":FieldChange::Remove {event_order:0,row,raw_key:b"transform".to_vec()}}}),
        );
        assert_eq!(
            result["ScalarSourceComparison"]["review"]["modeled_observations_match"], true,
            "{result}"
        );
        assert_eq!(
            result["ScalarSourceComparison"]["plan"]["edits"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(
            result["ScalarSourceComparison"]["review"]["revisions"][0]["content"]
                .as_str()
                .unwrap()
                .contains("-- keep")
        );
        let refused = request(
            json!({"CompareMonitorFieldSource":{"program":p,"change":FieldChange::Remove {event_order:0,row,raw_key:b"different".to_vec()}}}),
        );
        assert!(refused["Error"].is_string());
        let malformed = request(
            json!({"CompareMonitorFieldSource":{"program":p,"change":{"operation":"remove","event_order":0,"row":row,"raw_key":b"transform","unexpected":true}}}),
        );
        assert!(malformed["Error"].is_string());
    }
}
#[test]
fn declaration_sites_and_call_structure_changes_run_in_the_restricted_worker() {
    use hyprland_editor_core::{
        lua::replay::monitor_edit::declarations::{Anchor, Change},
        monitor, AUDITED_MAIN, BASELINE,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = replay_program("-- empty capture\n");
        p.version = version.into();
        let result = request(json!({"InspectMonitorDeclarations":{"program":p,"document":"root"}}));
        assert_eq!(result["MonitorDeclarationSites"]["runtime_verified"], false);
        assert_eq!(result["MonitorDeclarationSites"]["declarations"], json!([]));
        let mut rule = monitor::Rule::new("first");
        rule.output = monitor::Output::Connector("DP-1".into());
        let result = request(
            json!({"CompareMonitorDeclaration":{"program":p,"change":Change::Insert {document:"root".into(),anchor:Anchor::End,rule}}}),
        );
        assert_eq!(
            result["ScalarSourceComparison"]["review"]["modeled_observations_match"], true,
            "{result}"
        );
        assert_eq!(
            result["ScalarSourceComparison"]["review"]["structural_calls_matched"],
            1
        );
        let bad = request(
            json!({"CompareMonitorDeclaration":{"program":p,"change":{"operation":"remove","document":"root","span":{"start":0,"end":1},"unexpected":true}}}),
        );
        assert!(bad["Error"].is_string());
    }
}
#[test]
fn private_source_pieces_round_trip_inside_the_restricted_worker_without_rewriting_base() {
    use hyprland_editor_core::lua::{replay::edits::*, source_hash};
    let base = replay_program("hl.config({general={border_size=2}})");
    let doc = &base.documents[0];
    let start = doc.source.find('2').unwrap();
    let e = SourceEdit {
        document: doc.id.clone(),
        base_sha256: doc.sha256.clone(),
        span: start..start + 1,
        old_bytes_sha256: source_hash("2"),
        replacement: "6".into(),
        semantic_targets: vec!["general:border_size".into()],
    };
    let plan = prepare(&base, vec![e], vec![]).unwrap();
    let result = request(json!({"ApplySourceRevision":{"base":base,"revisions":[],"plan":plan}}));
    let revisions = result["SourceRevisions"].clone();
    assert!(revisions.is_array(), "{result}");
    let materialized =
        request(json!({"MaterializeSourceRevisions":{"base":base,"revisions":revisions}}));
    assert!(
        materialized["SourceRevisionProgram"]["documents"][0]["source"]
            .as_str()
            .unwrap()
            .contains("border_size=6")
    );
    assert!(base.documents[0].source.contains("border_size=2"));
    let mut forged = revisions.clone();
    forged[0]["sha256"] = "0".repeat(64).into();
    assert!(
        request(json!({"MaterializeSourceRevisions":{"base":base,"revisions":forged}}))["Error"]
            .is_string()
    );
}
#[test]
fn typed_source_comparisons_rebuild_coordinates_and_native_validation_in_the_worker() {
    use hyprland_editor_core::{
        lua::{
            config_inspection,
            replay::{scalar_edit::*, *},
        },
        Value,
    };
    let p = replay_program("hl.config({general={border_size=2}})");
    let report = replay(&p).unwrap();
    let inspection = config_inspection::inspect(&report.events, &p.version).unwrap();
    let c = &inspection.batches[0].candidates[0];
    let change = Change {
        event_order: c.event_order,
        location: c.location.clone(),
        setting_id: c.setting_id.clone(),
        selection: Selection::SelectedExpression,
        value: Value::Int(4),
    };
    let expected = serde_json::to_value(compare(&p, &change).unwrap()).unwrap();
    let actual = request(json!({"CompareScalarSource":{"program":p,"change":change}}));
    assert_eq!(actual["ScalarSourceComparison"], expected);
    let mut wrong = change.clone();
    wrong.location = vec![9];
    assert!(
        request(json!({"CompareScalarSource":{"program":p,"change":wrong}}))["Error"].is_string()
    );
}
#[test]
fn source_override_review_and_resolution_rebuild_evidence_in_the_restricted_worker() {
    use hyprland_editor_core::{lua::replay::override_review::*, Draft, Value};
    let p = replay_program("hl.config({general={border_size=2}})");
    let mut d = Draft::new(&p.version);
    d.values.insert("general:border_size".into(), Value::Int(7));
    let reviewed = request(json!({"ReviewSourceOverrides":{"program":p,"draft":d}}));
    let r: Review = serde_json::from_value(reviewed["SourceOverrideReview"].clone()).unwrap();
    assert!(r.model_complete);
    assert!(matches!(
        r.rows[0].source,
        SourceValue::Parsed {
            value: Value::Int(2),
            ..
        }
    ));
    let choices = [("general:border_size".to_string(), Choice::WriteOverride)].into();
    let expected = resolve(&p, &d, &r, &choices).unwrap();
    let actual = request(
        json!({"ResolveSourceOverrides":{"program":p,"draft":d,"review":r,"choices":choices}}),
    );
    assert_eq!(
        actual["SourceOverrideResolution"],
        serde_json::to_value(&expected).unwrap()
    );
    validate_resolution(&d, &r, &choices, &expected).unwrap();
    let mut forged = r.clone();
    forged.rows[0].override_value = Value::Int(3);
    assert!(request(
        json!({"ResolveSourceOverrides":{"program":p,"draft":d,"review":forged,"choices":choices}})
    )["Error"]
        .is_string());
    let mut other = d.clone();
    other
        .values
        .insert("general:border_size".into(), Value::Int(8));
    assert!(request(
        json!({"ResolveSourceOverrides":{"program":p,"draft":other,"review":r,"choices":choices}})
    )["Error"]
        .is_string());
}
#[test]
fn source_edit_review_uses_immutable_revisions_inside_the_restricted_worker() {
    use hyprland_editor_core::lua::{
        replay::{edits::*, *},
        source_hash,
    };
    let p = replay_program("hl.config({general={border_size=2}})");
    let d = &p.documents[0];
    let start = d.source.find('2').unwrap();
    let edit = SourceEdit {
        document: d.id.clone(),
        base_sha256: d.sha256.clone(),
        span: start..start + 1,
        old_bytes_sha256: source_hash("2"),
        replacement: "4".into(),
        semantic_targets: vec!["general:border_size".into()],
    };
    let event = &replay(&p).unwrap().events[0];
    let after = Observed::Table(vec![(
        Observed::Bytes(b"general".to_vec()),
        Observed::Table(vec![(
            Observed::Bytes(b"border_size".to_vec()),
            Observed::Int(4),
        )]),
    )]);
    let plan = prepare(
        &p,
        vec![edit],
        vec![ExpectedCall {
            order: 0,
            before_arguments: event.arguments.clone(),
            after_arguments: vec![after],
        }],
    )
    .unwrap();
    let expected = serde_json::to_value(review(&p, &plan).unwrap()).unwrap();
    let result = request(json!({"ReviewSourceEdits":{"program":p,"plan":plan}}));
    assert_eq!(result["SourceEditReview"], expected);
    assert_eq!(
        result["SourceEditReview"]["modeled_observations_match"],
        true
    );
    assert_eq!(result["SourceEditReview"]["ownership_verified"], false);
    assert_eq!(result["SourceEditReview"]["runtime_verified"], false);
    let mut bad = plan.clone();
    bad.edits[0].old_bytes_sha256 = source_hash("stale");
    assert!(request(json!({"ReviewSourceEdits":{"program":p,"plan":bad}}))["Error"].is_string());
}
#[test]
fn filesystem_and_network_are_denied() {
    let result = request(json!("Probe"));
    assert_eq!(result["Probe"]["file_denied"], true);
    assert_eq!(result["Probe"]["network_denied"], true);
}

fn replay_program(source: &str) -> hyprland_editor_core::lua::replay::Program {
    use hyprland_editor_core::lua::{replay::*, source_hash};
    Program {
        version: "0.56.2".into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            source: source.into(),
            sha256: source_hash(source),
            load_name: b"/private-fixture/main.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path_unknown_tail: false,
        initial_package_path: None,
        working_directory: None,
    }
}

#[test]
fn restricted_worker_keeps_partial_search_paths_distinct_from_literal_replacements() {
    use hyprland_editor_core::lua::replay::*;
    let mut p=replay_program("local saved=package.path;require('first');package.path='/vendor/?.lua';require('second');package.path=saved;require('third')");
    p.initial_package_path = Some(b"/private-fixture/?.lua;/private-fixture/?/init.lua".to_vec());
    p.initial_package_path_unknown_tail = true;
    let part = "return 1";
    p.documents.push(Document {
        id: "part".into(),
        source: part.into(),
        sha256: hyprland_editor_core::lua::source_hash(part),
        load_name: b"/private-fixture/part.lua".to_vec(),
    });
    for (module, unknown, path) in [
        ("first", true, p.initial_package_path.clone().unwrap()),
        ("second", false, b"/vendor/?.lua".to_vec()),
        ("third", true, p.initial_package_path.clone().unwrap()),
    ] {
        let reply = request(json!({"ReplayModules":p}));
        let pending: Request =
            serde_json::from_value(reply["Replayed"]["halt"]["request"].clone()).unwrap();
        assert_eq!(pending.module, module.as_bytes());
        assert_eq!(pending.package_path, Some(path));
        assert_eq!(pending.package_path_unknown_tail, unknown);
        assert_eq!(reply["Replayed"]["runtime_verified"], false);
        p.bindings.push(Binding {
            request: pending,
            resolution: Resolution::Loaded {
                document: "part".into(),
                loader_data: vec![],
            },
        });
    }
    assert!(request(json!({"ReplayModules":p}))["Replayed"]["halt"].is_null());
    p.initial_package_path = None;
    assert!(request(json!({"ReplayModules":p}))["Error"].is_string());
}

#[test]
fn ordered_modules_and_compilation_run_inside_the_restricted_worker() {
    use hyprland_editor_core::lua::{replay::*, source_hash};
    let mut p = replay_program(
        "local a=require('part');if a.n==2 then a.n=a.n+1;hl.config({general={border_size=require('part').n}})end",
    );
    let first = request(json!({"ReplayModules":p}));
    let request: Request =
        serde_json::from_value(first["Replayed"]["halt"]["request"].clone()).unwrap();
    assert!(first["Replayed"]["events"].as_array().unwrap().is_empty());
    let source = "return {n=2}";
    p.documents.push(Document {
        id: "part".into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: b"/private-fixture/part.lua".to_vec(),
    });
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/private-fixture/part.lua".to_vec(),
        },
    });
    let expected = serde_json::to_value(replay(&p).unwrap()).unwrap();
    let result = self::request(json!({"ReplayModules":p}));
    assert_eq!(result["Replayed"], expected);
    assert!(result["Replayed"]["halt"].is_null());
    assert_eq!(result["Replayed"]["events"].as_array().unwrap().len(), 1);
    assert_eq!(result["Replayed"]["runtime_verified"], false);
    assert_eq!(result["Replayed"]["events"][0]["path_prefix"], 1);
    assert_eq!(result["Replayed"]["path_decisions"][0]["kind"], "if");
    assert!(!result["Replayed"]["storage_writes"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        !result["Replayed"]["events"][0]["argument_sources"][0]["keys"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let bad = replay_program("hl.config({general={border_size=2}});local x <const> = 1;x=2");
    let result = self::request(json!({"ReplayModules":bad}));
    assert_eq!(result["Replayed"]["halt"]["kind"], "lua_error");
    assert_eq!(result["Replayed"]["events"], json!([]));
}

#[test]
fn replay_compiler_and_native_operations_never_execute_source() {
    let marker =
        std::env::temp_dir().join(format!("he-replay-never-execute-{}", std::process::id()));
    let p = replay_program(&format!(
        "os.execute('touch {}');hl.config({{general={{border_size=2}}}})",
        marker.display()
    ));
    let result = request(json!({"ReplayModules":p}));
    assert_eq!(result["Replayed"]["halt"]["kind"], "unresolved");
    assert_eq!(result["Replayed"]["events"], json!([]));
    assert!(!marker.exists());
    let denied = request(json!("Probe"));
    assert_eq!(denied["Probe"]["file_denied"], true);
    assert_eq!(denied["Probe"]["network_denied"], true);
}

#[test]
fn replay_budgets_bound_control_flow_and_alias_expansion_in_actual_worker() {
    for source in [
        "while true do end",
        "local t={v=1};for i=1,24 do t={t,t}end;hl.config(t)",
        "local function f()return f()end;pcall(f)",
    ] {
        let result = request(json!({"ReplayModules":replay_program(source)}));
        assert_eq!(result["Replayed"]["halt"]["kind"], "budget", "{result}");
        assert!(serde_json::to_vec(&result).unwrap().len() < 4 * 1024 * 1024);
    }
    let mut constants = String::from("return {");
    for i in 0..90_000 {
        use std::fmt::Write;
        write!(&mut constants, "k{i}=1,").unwrap();
    }
    constants.push('}');
    let result = request(json!({"ReplayModules":replay_program(&constants)}));
    assert_eq!(result["Replayed"]["halt"]["kind"], "budget");
    assert!(result["Replayed"]["halt"]["reason"]
        .as_str()
        .unwrap()
        .contains("compilation"));
    let name = "a".repeat(5000);
    let result = request(
        json!({"ReplayModules":replay_program(&format!("local {name} <const> = 1;{name}=2"))}),
    );
    assert_eq!(result["Replayed"]["halt"]["kind"], "budget");
    {
        use hyprland_editor_core::lua::{replay::*, source_hash};
        let mut p = replay_program("pcall(require,'huge');hl.config{never=true}");
        let source = format!("local {name} <const> = 1;{name}=2");
        p.documents.push(Document {
            id: "huge".into(),
            sha256: source_hash(&source),
            source,
            load_name: b"/private-fixture/huge.lua".to_vec(),
        });
        let pending = request(json!({"ReplayModules":p}));
        p.bindings.push(Binding {
            request: serde_json::from_value(pending["Replayed"]["halt"]["request"].clone())
                .unwrap(),
            resolution: Resolution::Loaded {
                document: "huge".into(),
                loader_data: b"/private-fixture/huge.lua".to_vec(),
            },
        });
        let result = request(json!({"ReplayModules":p}));
        assert_eq!(result["Replayed"]["halt"]["kind"], "budget");
        assert_eq!(result["Replayed"]["logged_module_errors"], 0);
        assert_eq!(result["Replayed"]["events"], json!([]));
    }
    let mut p = replay_program("hl.config({general={border_size=2}})");
    p.documents[0].sha256 = "0".repeat(64);
    assert!(request(json!({"ReplayModules":p}))["Error"].is_string());
}
#[test]
fn real_worker_imports_and_prepares_a_preserved_copy() {
    let source = "-- untouched\nhl.config({ ['general.border_size'] = 2 })\n";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    let mut report = parsed["Parsed"].clone();
    assert_eq!(report["draft"]["values"]["general:border_size"]["value"], 2);
    let mut draft = report["draft"].clone();
    draft["values"]["general:border_size"]["value"] = json!(5);
    let copy = request(json!({"Copy":{"source":source,"report":report.take(),"draft":draft}}));
    assert_eq!(copy["Copy"], source.replace("= 2", "= 5"));
}
#[test]
fn empty_statements_are_inert_in_the_actual_restricted_parser() {
    let source =
        ";local n=2;;hl.config{general={border_size=n}};;hl.config{decoration={rounding=n+1}};";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], false);
    assert_eq!(parsed["Parsed"]["diagnostics"], json!([]));
    assert_eq!(
        parsed["Parsed"]["draft"]["values"]["general:border_size"]["value"],
        2
    );
    assert_eq!(
        parsed["Parsed"]["draft"]["values"]["decoration:rounding"]["value"],
        3
    );
}
#[test]
fn opaque_commands_cannot_create_files() {
    let marker = std::env::temp_dir().join(format!(
        "hyprland-editor-must-not-create-{}",
        std::process::id()
    ));
    let source = format!(
        "os.execute('touch {}')\nhl.config({{['general.border_size']=3}})",
        marker.display()
    );
    let result = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(result["Parsed"]["opaque_effects"], true);
    assert!(!marker.exists());
    assert_eq!(result["Parsed"]["draft"]["values"], json!({}));
}
#[test]
fn calls_in_ignored_fields_are_inert_and_invalidate_later_api_trust() {
    let marker =
        std::env::temp_dir().join(format!("he-ignored-must-not-create-{}", std::process::id()));
    let source=format!("hl.config{{unknown=os.execute('touch {}'),general={{border_size=2}}}}\nhl.config{{general={{border_size=7}}}}",marker.display());
    let result = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(result["Parsed"]["opaque_effects"], true);
    assert_eq!(
        result["Parsed"]["draft"]["values"]["general:border_size"]["value"],
        2
    );
    assert!(!marker.exists());
}
#[test]
fn invalid_and_oversized_source_fail_without_partial_import() {
    let result = request(json!({"Parse":{"source":"hl.config({","version":"0.56.2"}}));
    assert!(result["Error"].as_str().unwrap().contains("syntax"));
    let result = request(json!({"Parse":{"source":"x".repeat(1024*1024+1),"version":"0.56.2"}}));
    assert!(result["Error"].as_str().unwrap().contains("1 MiB"));
}

#[test]
fn actual_restricted_worker_projects_monitors_and_keeps_unknown_calls_inert() {
    let source="hl.monitor{output='DP-1',mode='2560x1440@144',scale='1.25',position='-2048x0',reserved={top=-5}};hl.config{general={border_size=2}}";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["monitors"][0]["output"]["name"], "DP-1");
    assert_eq!(
        parsed["Parsed"]["monitors"][0]["fields"]["reserved"]["value"],
        json!([-5, 0, 0, 0])
    );
    assert_eq!(parsed["Parsed"]["opaque_effects"], false);
    let marker = std::env::temp_dir().join(format!("monitor-inert-{}", std::process::id()));
    let source = format!(
        "hl.monitor{{output='DP-1',mode=os.execute('touch {}')}};hl.monitor{{output='DP-2'}}",
        marker.display()
    );
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], true);
    assert_eq!(parsed["Parsed"]["monitors"], json!([]));
    assert!(!marker.exists());
}

#[test]
fn actual_worker_projects_devices_and_rejects_external_keyboard_entities() {
    let source="hl.device{name='USB keyboard',kb_layout='us,de',kb_variant='dvorak,',sensitivity=0.5};hl.monitor{output='DP-1'}";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["devices"][0]["name"], "USB keyboard");
    assert_eq!(
        parsed["Parsed"]["devices"][0]["fields"]["kb_variant"]["value"],
        "dvorak,"
    );
    assert_eq!(parsed["Parsed"]["opaque_effects"], false);
    let registry =
        request(json!({"ParseRegistry":{"xml":include_str!("fixtures/xkb-registry.xml")}}));
    assert_eq!(
        registry["Registry"]["layouts"][0]["variants"][0]["id"],
        "dvorak"
    );
    let xml="<!DOCTYPE xkbConfigRegistry [<!ENTITY leak SYSTEM 'file:///etc/passwd'>]><xkbConfigRegistry>&leak;</xkbConfigRegistry>";
    let result = request(json!({"ParseRegistry":{"xml":xml}}));
    assert!(result["Error"].is_string());
    let oversized = request(json!({"ParseRegistry":{"xml":" ".repeat(1024*1024+1)}}));
    assert!(oversized["Error"].is_string());
}
#[test]
fn installed_keyboard_registry_is_parsed_by_the_restricted_worker_when_available() {
    let path = std::path::Path::new("/usr/share/X11/xkb/rules/evdev.xml");
    if !path.is_file() {
        return;
    }
    let xml = hyprland_editor_core::storage::read_source(path)
        .unwrap()
        .content;
    let result = request(json!({"ParseRegistry":{"xml":xml}}));
    assert!(
        result["Registry"]["layouts"]
            .as_array()
            .is_some_and(|v| !v.is_empty()),
        "{result}"
    );
    assert!(result["Registry"]["groups"]
        .as_array()
        .is_some_and(|v| !v.is_empty()));
}

#[test]
fn binding_actions_remain_inert_in_actual_restricted_worker_and_preserve_timing() {
    let marker = std::env::temp_dir().join(format!("he-bind-never-execute-{}", std::process::id()));
    let source=format!("local action=hl.dsp.exec_cmd('touch {}');hl.define_submap('resize',function()hl.bind('R',function()hl.dispatch(action);hl.dispatch(hl.dsp.exit())end,{{release=true}})end)",marker.display());
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], false, "{parsed}");
    assert_eq!(
        parsed["Parsed"]["bindings"]["binds"][0]["actions"][0]["at_load"],
        true
    );
    assert_eq!(
        parsed["Parsed"]["bindings"]["binds"][0]["actions"][1]["at_load"],
        false
    );
    assert!(!marker.exists());
    let source=format!("hl.bind('Q',hl.dsp.no_op(),{{unknown=os.execute('touch {}')}});hl.bind('W',hl.dsp.no_op())",marker.display());
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], true);
    assert_eq!(parsed["Parsed"]["bindings"]["binds"], json!([]));
    assert!(!marker.exists());
}

#[test]
fn actual_restricted_worker_projects_workspace_rules_without_running_empty_hooks() {
    let marker =
        std::env::temp_dir().join(format!("he-workspace-never-run-{}", std::process::id()));
    let source=format!("hl.workspace_rule{{workspace='name:work',enabled=false,layout='master',layout_opts={{orientation='center',custom=false}},on_created_empty='touch {}'}}",marker.display());
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    assert_eq!(parsed["Parsed"]["opaque_effects"], false, "{parsed}");
    assert_eq!(parsed["Parsed"]["workspace_rules"][0]["enabled"], false);
    assert_eq!(
        parsed["Parsed"]["workspace_rules"][0]["layout_options"]["custom"]["value"],
        false
    );
    assert!(!marker.exists());
}

#[test]
fn bounded_re2_samples_run_inside_the_restricted_worker() {
    for (pattern, text, negated, expected) in [
        ("fire.*", "firefox", false, true),
        ("fire", "firefox", false, false),
        ("fire", "firefox", true, true),
        ("δοκιμή", "δοκιμή", false, true),
    ] {
        let result =
            request(json!({"CheckRegex":{"pattern":pattern,"text":text,"negated":negated}}));
        assert_eq!(result["RegexMatch"], expected, "{result}");
    }
    for pattern in ["[", "(?=x)", "(x)\\1"] {
        assert!(
            request(json!({"CheckRegex":{"pattern":pattern,"text":"x","negated":false}}))["Error"]
                .is_string()
        );
    }
    assert!(
        request(json!({"CheckRegex":{"pattern":"x".repeat(16385),"text":"","negated":false}}))
            ["Error"]
            .is_string()
    );
}

#[test]
fn actual_restricted_worker_projects_inert_rule_groups_and_preserves_copy() {
    let source="hl.window_rule{name='test',enabled=false,match={class='negative:foo.*'},move={'monitor_w - window_w',20}};hl.window_rule{name='test',center=true};hl.layer_rule{match={namespace='panel'},blur=false};hl.config{general={border_size=2}}";
    let parsed = request(json!({"Parse":{"source":source,"version":"0.56.2"}}));
    let report = &parsed["Parsed"];
    assert_eq!(report["opaque_effects"], false);
    assert_eq!(report["rules"].as_array().unwrap().len(), 3);
    assert_eq!(report["rules"][0]["enabled"], false);
    assert_eq!(report["rules"][1]["enabled"], Value::Null);
    let mut draft = report["draft"].clone();
    draft["values"]["general:border_size"]["value"] = json!(7);
    let copy = request(json!({"Copy":{"source":source,"report":report,"draft":draft}}));
    assert_eq!(
        copy["Copy"],
        source.replace("border_size=2", "border_size=7")
    );
    let tainted = request(
        json!({"Parse":{"source":"hl.window_rule{move={os.execute('never execute'),2}};hl.layer_rule{blur=true}","version":"0.56.2"}}),
    );
    assert_eq!(tainted["Parsed"]["opaque_effects"], true);
    assert_eq!(tainted["Parsed"]["rules"], json!([]));
}

#[test]
fn module_candidate_plans_use_only_supplied_bytes_and_reject_amplification() {
    let input = json!({"PlanModules":{
        "context":{"main_file":b"/synthetic/config/main.lua","home":null,"inherited_package_path":null},
        "modules":[b"./theme".to_vec(),b"named.theme".to_vec()],"version":"0.56.2"
    }});
    let response = request(input.clone());
    let plans = response["ModulePlans"].as_array().unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0]["primary"]["kind"], "explicit_files");
    assert_eq!(
        plans[0]["primary"]["base"],
        json!(b"/synthetic/config/theme")
    );
    assert_eq!(plans[1]["inherited_search_unknown"], true);
    let mut original = input.clone();
    original["PlanModules"]["modules"] = json!([b"./parts/*".to_vec()]);
    original["PlanModules"]["mode"] = json!("original");
    assert_eq!(
        request(original.clone())["ModulePlans"][0]["primary"]["kind"],
        "named"
    );
    original["PlanModules"]["mode"] = json!("safe");
    assert_eq!(
        request(original)["ModulePlans"][0]["primary"]["kind"],
        "explicit_wildcard"
    );
    let mut denied = input;
    denied["PlanModules"]["context"]["inherited_package_path"] = json!(b"/x/?.lua;".repeat(500));
    denied["PlanModules"]["modules"] = json!(vec![vec![b'a'; 4096]]);
    assert!(request(denied)["Error"].as_str().unwrap().contains("1 MiB"));
}

#[test]
fn dependency_inspection_reports_scopes_and_aliases_without_running_config_or_opening_targets() {
    let marker = std::env::temp_dir().join(format!("he-dependency-marker-{}", std::process::id()));
    assert!(!marker.exists());
    let source=format!("local r=require;local p='./theme';r(p);local callback=function()dofile('/etc/passwd')end;os.execute('touch {}')",marker.display());
    let response = request(json!({"InspectDependencies":{"source":source,"version":"0.56.2"}}));
    let inventory: hyprland_editor_core::lua::DependencyInventory =
        serde_json::from_value(response["Dependencies"].clone()).unwrap();
    inventory.validate(&source, "0.56.2").unwrap();
    assert_eq!(inventory.sites.len(), 2);
    assert_eq!(inventory.sites[0].module.as_deref(), Some("./theme"));
    assert_eq!(
        inventory.sites[0].binding,
        hyprland_editor_core::lua::DependencyBinding::CapturedCandidate
    );
    assert_eq!(
        inventory.sites[1].scope,
        hyprland_editor_core::lua::DependencyScope::FunctionBody
    );
    assert_eq!(inventory.sites[1].module.as_deref(), Some("/etc/passwd"));
    assert!(!marker.exists());
}

#[test]
fn primitive_coercion_stays_in_the_restricted_worker_and_refuses_batch_budgets() {
    use hyprland_editor_core::lua::{
        replay::Observed,
        scalar_codec::{Case, Codec},
    };
    let marker =
        std::env::temp_dir().join(format!("he-coercion-never-execute-{}", std::process::id()));
    let script = format!("os.execute('touch {}')", marker.display());
    let cases = vec![
        Case {
            codec: Codec::Bool,
            input: Observed::Bytes(b" 0x1p0 ".to_vec()),
        },
        Case {
            codec: Codec::Float {
                minimum: None,
                maximum: None,
            },
            input: Observed::Int(16777217),
        },
        Case {
            codec: Codec::String { validator: None },
            input: Observed::Bytes(script.as_bytes().to_vec()),
        },
        Case {
            codec: Codec::String {
                validator: Some("unknown".into()),
            },
            input: Observed::Bytes(b"text".to_vec()),
        },
    ];
    let mut cases = cases;
    for bits in [0xfff8000000000000, 0x7ff0000000000000] {
        cases.push(Case {
            codec: Codec::Float {
                minimum: None,
                maximum: None,
            },
            input: Observed::Float(f64::from_bits(bits)),
        });
    }
    let reply = request(json!({"DecodePrimitives":cases}));
    let values = &reply["Primitives"]["outcomes"];
    assert_eq!(values[0]["native"]["value"], true, "{reply}");
    assert_eq!(values[1]["native"]["value"], 0x4b800000u32, "{reply}");
    assert_eq!(
        values[2]["representation"]["value"]["value"], script,
        "{reply}"
    );
    assert_eq!(values[3]["kind"], "unresolved", "{reply}");
    assert_eq!(values[4]["native"]["value"], 0xffc00000u32, "{reply}");
    assert_eq!(values[5]["native"]["value"], 0x7f800000u32, "{reply}");
    assert_eq!(
        values[4]["representation"]["kind"], "unavailable",
        "{reply}"
    );
    assert_eq!(reply["Primitives"]["numeric_locale"], "C");
    assert!(reply["Primitives"]["profile"]["release"].is_string());
    assert!(!marker.exists(), "Primitive text was executed");
    assert!(request(json!({"DecodePrimitives":[]}))["Error"]
        .as_str()
        .unwrap()
        .contains("1–256"));
    let small = Case {
        codec: Codec::Bool,
        input: Observed::Bool(false),
    };
    assert!(request(json!({"DecodePrimitives":vec![small;257]}))["Error"].is_string());
    let large = Case {
        codec: Codec::String { validator: None },
        input: Observed::Bytes(vec![b'x'; 65536]),
    };
    assert!(request(json!({"DecodePrimitives":vec![large;9]}))["Error"]
        .as_str()
        .unwrap()
        .contains("2 MiB"));
    assert!(!marker.exists());
}

#[test]
fn monitor_source_draft_review_and_resolution_stay_in_the_restricted_worker() {
    use hyprland_editor_core::{
        lua::replay::monitor_review::{Choice, Review},
        monitor::{Output, Rule},
        Value, AUDITED_MAIN, BASELINE,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let mut program = replay_program(
            "hl.monitor{output='DP-1',transform=1};hl.monitor{output='DP-1',transform=2}",
        );
        program.version = version.into();
        let mut rule = Rule::new("draft");
        rule.output = Output::Connector("DP-1".into());
        rule.fields.insert("transform".into(), Value::Int(3));
        let monitors = vec![rule];
        let result =
            request(json!({"ReviewSourceMonitors":{"program":program,"monitors":monitors}}));
        let review: Review = serde_json::from_value(result["SourceMonitorReview"].clone()).unwrap();
        assert_eq!(review.sources.len(), 2);
        assert!(review.model_complete);
        let result = request(
            json!({"ResolveSourceMonitors":{"program":program,"monitors":monitors,"review":review,"choice":Choice::WriteDraft {draft_id:"draft".into(),event_order:0}}}),
        );
        let result = &result["SourceMonitorResolution"];
        assert_eq!(result["ownership_verified"], false);
        assert_eq!(result["runtime_verified"], false);
        assert_eq!(result["plans"].as_array().unwrap().len(), 1);
        assert_eq!(result["previews"][0]["before"], "1");
        assert_eq!(result["previews"][0]["after"], "3");
        let result = request(
            json!({"ResolveSourceMonitors":{"program":program,"monitors":monitors,"review":review,"choice":Choice::CopySource {event_order:1,replace_id:None}}}),
        );
        assert_eq!(
            result["SourceMonitorResolution"]["monitors"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            result["SourceMonitorResolution"]["plans"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}

#[test]
fn scalar_override_reconciliation_after_monitors_uses_the_restricted_complete_trace() {
    use hyprland_editor_core::{
        lua::replay::override_review::{Choice, Review},
        Draft, Value, AUDITED_MAIN, BASELINE,
    };
    for version in [BASELINE, AUDITED_MAIN] {
        let mut program=replay_program("hl.monitor{output='DP-1',transform=1};hl.config{general={border_size=2}};hl.monitor{output='DP-2'};hl.config{general={border_size=3}}");
        program.version = version.into();
        let mut draft = Draft::new(version);
        draft
            .values
            .insert("general:border_size".into(), Value::Int(9));
        let response = request(json!({"ReviewSourceOverrides":{"program":program,"draft":draft}}));
        let review: Review =
            serde_json::from_value(response["SourceOverrideReview"].clone()).unwrap();
        assert!(review.model_complete);
        assert_eq!(review.rows[0].declaration_count, 2);
        assert_eq!(review.rows[0].target.as_ref().unwrap().event_order, 3);
        let response = request(
            json!({"ResolveSourceOverrides":{"program":program,"draft":draft,"review":review,"choices":{"general:border_size":Choice::WriteOverride}}}),
        );
        let result = &response["SourceOverrideResolution"];
        assert_eq!(result["model_complete"], true);
        assert_eq!(result["remaining"], 0);
        assert_eq!(result["plans"][0]["expected_calls"][0]["order"], 3);
        assert_eq!(result["ownership_verified"], false);
        assert_eq!(result["runtime_verified"], false);
    }
}
