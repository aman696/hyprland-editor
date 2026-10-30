use hyprland_editor_core::{
    catalogue, legacy, legacy_monitor as codec,
    monitor::{self, LegacyOperation as Op, Output, Rule},
    session, Value, AUDITED_MAIN, BASELINE,
};
const SOURCE: &str = "# display café\r\nmonitor = DP-1,2560x1440@144,0x0,1,transform,1,bitdepth,10,cm,srgb,vrr,1,sdrbrightness,1.2,sdrsaturation,0.9,mirror,HDMI-A-1,icc,/profiles/screen.icc # replace\r\nmonitor = HDMI-A-1,disabled # disable\r\nmonitor = DP-1,transform,3 # transform only\r\nmonitor = DP-1,addreserved,10,20,30,40 # reserved only\r\nsource = missing.conf\r\n";
#[test]
fn legacy_monitor_operations_keep_their_distinct_intentions_and_source_bytes() {
    let report = codec::inspect(SOURCE).unwrap();
    assert_eq!(report.rules.len(), 4);
    assert_eq!(
        report
            .rules
            .iter()
            .map(|r| r.legacy_operation.unwrap())
            .collect::<Vec<_>>(),
        Op::ALL
    );
    assert_eq!(report.rules[0].fields.len(), 11);
    assert_eq!(
        report.rules[3].fields["reserved"],
        Value::CssGap([10, 40, 20, 30])
    );
    assert_eq!(codec::edited_copy(SOURCE, &report.rules).unwrap(), SOURCE);
    for rule in &report.rules {
        assert!(rule.export_lua(BASELINE).is_err());
        assert!(rule.validate(AUDITED_MAIN).is_err());
    }
    let mut rules = report.rules;
    rules[0]
        .fields
        .insert("mode".into(), Value::String("1920x1080@60".into()));
    rules[2].fields.insert("transform".into(), Value::Int(5));
    rules[3]
        .fields
        .insert("reserved".into(), Value::CssGap([1, 2, 3, 4]));
    let output = codec::edited_copy(SOURCE, &rules).unwrap();
    assert!(output.contains("monitor = DP-1,transform,5 # transform only"));
    assert!(output.contains("monitor = DP-1,addreserved,1,3,4,2 # reserved only"));
    assert!(output.ends_with("source = missing.conf\r\n"));
    assert!(output.contains("# replace\r\n") && output.contains("# disable\r\n"));
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().rules).unwrap(),
        codec::export(&rules).unwrap()
    );
}
#[test]
fn monitor_reorder_duplicate_add_remove_and_scalar_session_edits_reparse_together() {
    let source =
        format!("{SOURCE}general:border_size = 2 # scalar\r\nenv = THEME,Nord # environment\r\n");
    let doc = legacy::Document::new(source.clone(), &catalogue()).unwrap();
    let mut rules = doc.monitors.rules.clone();
    rules.swap(1, 2);
    rules.remove(3);
    let mut duplicate = rules[0].clone();
    duplicate.id = "copy".into();
    rules.push(duplicate);
    let mut added = Rule::new("new");
    added.output = Output::Fallback;
    added.legacy_operation = Some(Op::Replace);
    rules.push(added);
    let mut scalar = doc.report.draft.clone();
    scalar
        .values
        .insert("general:border_size".into(), Value::Int(8));
    let session = session::Draft {
        entries: vec![session::Entry::Environment {
            name: "THEME".into(),
            value: "New#theme".into(),
            dbus: Some(true),
        }],
    };
    let output = doc
        .edited_with_monitors(&scalar, &session, &rules, &catalogue())
        .unwrap();
    assert_eq!(
        codec::export(&codec::inspect(&output).unwrap().rules).unwrap(),
        codec::export(&rules).unwrap()
    );
    assert!(output.contains("border_size = 8 # scalar"));
    assert_eq!(
        hyprland_editor_core::legacy_session::inspect(&output)
            .unwrap()
            .draft,
        session
    );
    let removed = doc
        .edited_with_monitors(&scalar, &session, &[], &catalogue())
        .unwrap();
    assert!(codec::inspect(&removed).unwrap().rules.is_empty());
    assert!(removed.contains("# transform only") && removed.contains("# reserved only"));
    assert_eq!(doc.content, source);
}
#[test]
fn literal_selectors_modelines_empty_defaults_and_escaped_arguments_roundtrip() {
    for name in [
        "",
        "DP-1",
        "desc:café, screen #1",
        "desc:back\\slash {screen}",
    ] {
        let mut rule = Rule::new("test");
        rule.legacy_operation = Some(Op::Replace);
        rule.output = if name.is_empty() {
            Output::Fallback
        } else if let Some(s) = name.strip_prefix("desc:") {
            Output::Description(s.into())
        } else {
            Output::Connector(name.into())
        };
        rule.fields.insert(
            "mode".into(),
            Value::String(
                "modeline 173 1920 2048 2248 2576 1080 1083 1088 1120 +hsync -vsync".into(),
            ),
        );
        let source = codec::encode(&rule).unwrap();
        let report = codec::inspect(&source).unwrap();
        assert_eq!(report.rules.len(), 1, "{source}");
        assert_eq!(report.rules[0].output, rule.output);
        assert_eq!(codec::encode(&report.rules[0]).unwrap(), source);
    }
    let defaults = codec::inspect("monitor = ,,,\n").unwrap();
    assert_eq!(
        defaults.rules[0].fields["mode"],
        Value::String("preferred".into())
    );
    assert_eq!(
        defaults.rules[0].fields["position"],
        Value::String("auto".into())
    );
    assert_eq!(
        defaults.rules[0].fields["scale"],
        Value::String("auto".into())
    );
}
#[test]
fn unresolved_monitor_source_is_retained_and_lua_intent_is_never_silently_converted() {
    let source="monitor = DP-1,preferred,auto,1\nmonitor = $display,preferred,auto,1\nmonitor = DP-2,preferred,auto,1,unknown,value\ngeneral {\nmonitor = nested,preferred,auto,1\n}\n# hyprlang if something\nmonitor = conditional,preferred,auto,1\n";
    let report = codec::inspect(source).unwrap();
    assert_eq!(report.rules.len(), 1);
    assert_eq!(report.unresolved_count, 4);
    let output = codec::edited_copy(source, &[]).unwrap();
    for line in source.lines().skip(1) {
        assert!(output.contains(line));
    }
    let mut lua = Rule::new("lua");
    lua.fields
        .insert("mode".into(), Value::String("preferred".into()));
    assert!(codec::encode(&lua).is_err());
    lua.legacy_operation = Some(Op::Transform);
    assert!(lua.validate(BASELINE).is_err());
    lua.fields.clear();
    assert!(codec::encode(&lua).is_err());
    lua.fields.insert("transform".into(), Value::Int(8));
    assert!(codec::encode(&lua).is_err());
    assert!(monitor::validate(&vec![Rule::new("same"); 65], BASELINE).is_err());
}
#[test]
fn private_legacy_monitor_drafts_require_their_exact_source_and_codec_markers() {
    use hyprland_editor_core::{
        source::{RecordedOwnership, SourceIdentity},
        workspace::{SourceLanguage, Workspace},
        Draft,
    };
    let source = "monitor = DP-1,transform,1\n";
    let hash = hyprland_editor_core::lua::source_hash(source);
    let identity = SourceIdentity {
        selected_path: b"/never-opened/config.conf".to_vec(),
        resolved_path: None,
        links: vec![],
        device: 1,
        inode: 2,
        owner: 1000,
        mode: 0o100600,
        size: source.len() as u64,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
        sha256: hash.clone(),
        recorded_ownership: RecordedOwnership::UserOwned,
    };
    let mut workspace = Workspace::new(Draft::new(BASELINE))
        .with_root(source.into(), identity)
        .with_monitors(codec::inspect(source).unwrap().rules);
    workspace.sources[0].language = SourceLanguage::Legacy;
    assert!(workspace.validate(&catalogue()).is_err());
    workspace.legacy_monitor_origin_sha256 = Some(hash);
    workspace.legacy_monitor_schema_sha256 = Some(codec::schema_hash());
    let bytes = workspace.to_json(&catalogue()).unwrap();
    let reopened =
        Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
    assert_eq!(reopened.monitors, workspace.monitors);
    workspace.monitors.clear();
    assert!(workspace.validate(&catalogue()).is_ok());
    workspace.legacy_monitor_schema_sha256 = Some("0".repeat(64));
    assert!(workspace.validate(&catalogue()).is_err());
}
