use codec::{Chord, Draft, Entry};
use hyprland_editor_core::{
    binding, catalogue, legacy, legacy_binding as codec,
    lua::source_hash,
    session,
    workspace::{SourceLanguage, SourceSnapshot, Workspace},
    Draft as Scalars, Value, AUDITED_MAIN, BASELINE,
};
const SOURCE:&str="# café\r\nbind = SUPER,Return,exec,application,with,commas # launch\r\nbinddk = CTRL,Q,!keyboard-one keyboard-two,Close app,killactive, # filter\r\nbindm = SUPER,mouse:272,movewindow # mouse\r\nsubmap = resize,reset # scope\r\nbinds = Control_L,Left&Right,movefocus,l # multiple\r\nbind = ,catchall,submap,reset # escape\r\nunbind = SUPER,42 # remove code\r\nsubmap = reset # global\r\nunbind = all # clear\r\n";
#[test]
fn native_binding_variants_keep_order_flags_devices_and_flat_submap_scope() {
    let report = codec::inspect(SOURCE).unwrap();
    assert_eq!(report.unresolved_count, 0);
    assert_eq!(report.draft.entries.len(), 9);
    report.draft.validate(BASELINE).unwrap();
    assert_eq!(codec::edited_copy(SOURCE, &report.draft).unwrap(), SOURCE);
    assert!(
        matches!(&report.draft.entries[0],Entry::Bind{argument,..}if argument=="application,with,commas")
    );
    assert!(
        matches!(&report.draft.entries[1],Entry::Bind{description:Some(d),device:Some(f),..}if d=="Close app"&&!f.inclusive&&f.items==["keyboard-one","keyboard-two"])
    );
    assert!(
        matches!(&report.draft.entries[2],Entry::Bind{dispatcher,argument,flags,..}if dispatcher=="mouse"&&argument=="movewindow"&&flags.contains(&'m'))
    );
    assert!(
        matches!(&report.draft.entries[4],Entry::Bind{chord,..}if chord.multi&&chord.keys==["Left","Right"])
    );
    assert!(matches!(&report.draft.entries[6],Entry::Unbind{chord,..}if chord.keys==["code:42"]));
    let exported = codec::export(&report.draft).unwrap();
    assert_eq!(codec::inspect(&exported).unwrap().draft, report.draft);
    assert!(report.draft.validate(AUDITED_MAIN).is_err());
}
#[test]
fn combined_source_editing_reorders_duplicates_removes_and_retains_other_families() {
    let source=format!("{SOURCE}general:border_size = 2 # scalar\r\nenv = THEME,Nord # env\r\nmonitor = DP-1,transform,1 # monitor\r\n");
    let doc = legacy::Document::new(source.clone(), &catalogue()).unwrap();
    let mut target = doc.bindings.draft.clone();
    target.entries.swap(0, 1);
    target.entries.remove(8);
    target.entries.push(target.entries[0].clone());
    if let Entry::Bind { argument, .. } = &mut target.entries[1] {
        *argument = "changed ## command".into();
    }
    let mut scalar = doc.report.draft.clone();
    scalar
        .values
        .insert("general:border_size".into(), Value::Int(6));
    let session = session::Draft {
        entries: vec![session::Entry::Environment {
            name: "THEME".into(),
            value: "New".into(),
            dbus: Some(false),
        }],
    };
    let mut monitors = doc.monitors.rules.clone();
    monitors[0].fields.insert("transform".into(), Value::Int(3));
    let output = doc
        .edited_with_bindings(&scalar, &session, &monitors, &target, &catalogue())
        .unwrap();
    assert_eq!(codec::inspect(&output).unwrap().draft, target);
    assert!(output.contains("border_size = 6 # scalar\r\n"));
    assert!(output.contains("monitor = DP-1,transform,3 # monitor\r\n"));
    assert!(output.contains("THEME,New") && output.contains("# env\r\n"));
    for comment in [
        "# launch",
        "# filter",
        "# mouse",
        "# scope",
        "# multiple",
        "# escape",
        "# remove code",
        "# global",
        "# clear",
    ] {
        assert!(output.contains(comment));
    }
    let removed = codec::edited_copy(&source, &Draft::default()).unwrap();
    assert!(codec::inspect(&removed).unwrap().draft.is_empty());
    assert!(removed.contains("# escape\r\n"));
    assert_eq!(doc.content, source);
}
#[test]
fn unresolved_submap_and_include_context_are_retained_and_unsafe_changes_refused() {
    let source="submap = resize\nbind = ,R,exec,known\nbind = $modifier,Q,exec,unknown\nsource = missing.conf\nbind = ,X,exec,scope-unknown\nsubmap = reset\nbind = ,A,killactive,\n";
    let before = codec::inspect(source).unwrap();
    assert_eq!(before.draft.entries.len(), 4);
    assert_eq!(before.unresolved_count, 3);
    let mut changed = before.draft.clone();
    if let Entry::Bind { argument, .. } = &mut changed.entries[1] {
        *argument = "changed".into();
    }
    let output = codec::edited_copy(source, &changed).unwrap();
    assert!(output.contains("bind = $modifier,Q,exec,unknown\n"));
    assert!(output.contains("bind = ,X,exec,scope-unknown\n"));
    let mut unsafe_scope = changed.clone();
    if let Entry::Submap { name, .. } = &mut unsafe_scope.entries[0] {
        *name = "other".into();
    }
    assert!(codec::edited_copy(source, &unsafe_scope).is_err());
    let dynamic = "$tail = ubmap\ns$tail = resize\nbind = ,R,exec,unknown-scope\nsubmap = reset\nbind = ,Q,killactive,\n";
    let report = codec::inspect(dynamic).unwrap();
    assert_eq!(report.unresolved_count, 2);
    assert_eq!(report.draft.entries.len(), 2);
    assert!(codec::edited_copy(dynamic, &report.draft)
        .unwrap()
        .contains("bind = ,R,exec,unknown-scope"));
    let source = "source = unknown.conf\n";
    let binding = codec::inspect("bind = SUPER,R,exec,app\n")
        .unwrap()
        .draft
        .entries
        .remove(0);
    let mut appended = Draft {
        entries: vec![binding.clone()],
    };
    assert!(codec::edited_copy(source, &appended).is_err());
    appended.entries.insert(
        0,
        Entry::Submap {
            name: String::new(),
            reset: None,
        },
    );
    assert!(codec::edited_copy(source, &appended)
        .unwrap()
        .contains("submap = reset"));
    assert_eq!(
        codec::inspect("bind = ,catchall,exec,app\n")
            .unwrap()
            .unresolved_count,
        1
    );
}
#[test]
fn argument_literals_flags_and_all_dispatcher_names_roundtrip_without_dispatch() {
    for argument in [
        "",
        "a,b,c",
        "café",
        "#",
        "###",
        "quoted 'value'",
        r"a\b",
        "{{literal}}",
        "^end$",
    ] {
        let mut draft = codec::inspect("bind = SUPER,R,exec,app\n").unwrap().draft;
        if let Entry::Bind { argument: a, .. } = &mut draft.entries[0] {
            *a = argument.into();
        }
        let text = codec::export(&draft).unwrap();
        assert_eq!(codec::inspect(&text).unwrap().draft, draft, "{text}");
    }
    assert_eq!(codec::dispatchers().len(), 71);
    for dispatcher in codec::dispatchers() {
        let source = format!("bind = ,R,{dispatcher},\n");
        assert_eq!(
            codec::inspect(&source).unwrap().draft.entries.len(),
            1,
            "{dispatcher}"
        );
    }
    for suffix in ["er", "eo", "em", "ml", "mc", "cg", "unknown"] {
        assert!(
            codec::inspect(&format!("bind{suffix} = SUPER,R,exec,app\n"))
                .unwrap()
                .draft
                .is_empty()
        );
    }
    let mut entry = codec::inspect("bind = ,R,exec,app\n")
        .unwrap()
        .draft
        .entries
        .remove(0);
    if let Entry::Bind { argument, .. } = &mut entry {
        *argument = "$program".into();
    }
    assert!(codec::encode(&entry).is_err());
    assert!(Entry::Submap {
        name: "reset".into(),
        reset: None
    }
    .validate()
    .is_err());
    // The mask check also applies to multi-key syntax in the native handler.
    assert!(codec::inspect("binds = F1,R,exec,app\n")
        .unwrap()
        .draft
        .is_empty());
}
#[test]
fn legacy_project_markers_and_binding_undo_preserve_intent_without_lua_conversion() {
    let legacy = codec::inspect(SOURCE).unwrap().draft;
    let before = binding::Draft {
        legacy: Some(legacy),
        ..Default::default()
    };
    let after = binding::Draft::default();
    assert!(before.export_lua(BASELINE).is_err());
    let inverse = binding::Inverse::between(&before, &after);
    let mut restored = after;
    inverse.restore(&mut restored);
    assert_eq!(restored, before);
    let mut workspace = Workspace::new(Scalars::new(BASELINE)).with_bindings(before);
    assert!(workspace.validate(&catalogue()).is_err());
    let origin = hyprland_editor_core::source::SourceIdentity {
        selected_path: b"/never-opened/config.conf".to_vec(),
        resolved_path: None,
        links: vec![],
        device: 1,
        inode: 2,
        owner: 1000,
        mode: 0o100600,
        size: SOURCE.len() as u64,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
        sha256: source_hash(SOURCE),
        recorded_ownership: hyprland_editor_core::source::RecordedOwnership::UserOwned,
    };
    workspace.root_source = Some("root".into());
    workspace.sources.push(SourceSnapshot {
        id: "root".into(),
        language: SourceLanguage::Legacy,
        content: SOURCE.into(),
        origin,
    });
    workspace.legacy_binding_origin_sha256 = Some(source_hash(SOURCE));
    workspace.legacy_binding_schema_sha256 = Some(codec::schema_hash());
    let bytes = workspace.to_json(&catalogue()).unwrap();
    assert_eq!(
        Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue())
            .unwrap()
            .bindings,
        workspace.bindings
    );
    workspace.bindings.legacy = Some(Draft::default());
    let bytes = workspace.to_json(&catalogue()).unwrap();
    let reopened =
        Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
    assert!(reopened.bindings.is_empty());
    assert!(reopened.legacy_binding_origin_sha256.is_some());
    workspace.legacy_binding_schema_sha256 = Some("0".repeat(64));
    assert!(workspace.validate(&catalogue()).is_err());
}
#[test]
fn unknown_binding_reports_remain_bounded_and_never_capture_host_variables() {
    let source = "bind = $HOSTVALUE,R,exec,app\n".repeat(20000);
    let report = codec::inspect(&source).unwrap();
    assert_eq!(report.unresolved_count, 20000);
    assert_eq!(report.unresolved.len(), 4096);
    assert!(report.draft.is_empty());
    assert_eq!(report.barriers_sha256.len(), 64);
    assert!(serde_json::to_vec(&report).unwrap().len() < 65536);
    assert_eq!(
        codec::edited_copy(&source, &Draft::default()).unwrap(),
        source
    );
    let huge = Draft {
        entries: vec![
            Entry::Unbind {
                all: false,
                chord: Chord {
                    keys: vec!["R".into()],
                    ..Default::default()
                }
            };
            10001
        ],
    };
    assert!(huge.validate(BASELINE).is_err());
}
