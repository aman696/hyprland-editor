use hyprland_editor_core::{
    catalogue, legacy, legacy_session,
    session::{self, Draft, Entry, PermissionMode, PermissionType, Trigger},
    workspace::Workspace,
    Value, AUDITED_MAIN, BASELINE,
};
const SOURCE:&str="# café\r\nenv = GTK_THEME,Nord # environment\r\nenvd = EMPTY,\r\nexec = [float] application # normal\r\nexecr = raw application\r\nexec-once = startup application\r\nexecr-once = raw startup\r\nexec-shutdown = cleanup application\r\npermission = /usr/bin/app,screencopy,ask # policy\r\nsource = unknown.conf\r\n";
#[test]
fn all_legacy_session_directives_preserve_order_raw_modes_and_unchanged_bytes() {
    let doc = legacy::Document::new(SOURCE.into(), &catalogue()).unwrap();
    assert_eq!(doc.session.draft.entries.len(), 8);
    assert_eq!(doc.session.unresolved_count, 0);
    assert!(matches!(
        &doc.session.draft.entries[2],
        Entry::LegacyCommand {
            raw: false,
            trigger: Trigger::ConfigLoad,
            ..
        }
    ));
    assert!(matches!(
        &doc.session.draft.entries[3],
        Entry::LegacyCommand { raw: true, .. }
    ));
    assert_eq!(
        doc.edited_with_session(&doc.report.draft, &doc.session.draft, &catalogue())
            .unwrap(),
        SOURCE
    );
    let mut draft = doc.session.draft.clone();
    if let Entry::Environment { value, .. } = &mut draft.entries[0] {
        *value = "Changed".into();
    }
    if let Entry::LegacyCommand { command, raw, .. } = &mut draft.entries[2] {
        *command = "new application".into();
        *raw = true;
    }
    let output = doc
        .edited_with_session(&doc.report.draft, &draft, &catalogue())
        .unwrap();
    assert!(output.contains("env = GTK_THEME,Changed"));
    assert!(output.contains("execr = new application"));
    assert!(output.contains("# environment\r\n"));
    assert!(output.contains("# normal\r\n"));
    assert!(output.ends_with("source = unknown.conf\r\n"));
    let reparsed = legacy_session::inspect(&output).unwrap();
    assert_eq!(reparsed.draft, draft);
}
#[test]
fn scalar_and_session_edits_insert_duplicate_reorder_remove_and_reparse_together() {
    let source = format!("{SOURCE}general {{\r\n border_size = 2 # keep scalar\r\n}}\r\n");
    let doc = legacy::Document::new(source.clone(), &catalogue()).unwrap();
    let mut scalar = doc.report.draft.clone();
    scalar
        .values
        .insert("general:border_size".into(), Value::Int(7));
    let mut draft = doc.session.draft.clone();
    draft.entries.swap(0, 1);
    draft.entries.remove(3);
    draft.entries.push(Entry::Environment {
        name: "NEW_VALUE".into(),
        value: "a,b,#tag".into(),
        dbus: Some(true),
    });
    draft.entries.push(draft.entries[0].clone());
    let output = doc
        .edited_with_session(&scalar, &draft, &catalogue())
        .unwrap();
    assert!(output.contains("border_size = 7 # keep scalar"));
    assert_eq!(legacy_session::inspect(&output).unwrap().draft, draft);
    assert!(output.contains("source = unknown.conf"));
    assert_eq!(doc.content, source);
    let empty = Draft::default();
    let removed = doc
        .edited_with_session(&scalar, &empty, &catalogue())
        .unwrap();
    assert!(legacy_session::inspect(&removed).unwrap().draft.is_empty());
    assert!(removed.contains("# normal") && removed.contains("# policy"));
    assert!(removed.contains("source = unknown.conf"));
}
#[test]
fn pinned_hash_and_escape_rules_match_golden_boundaries_and_keep_literal_values() {
    for (raw, decoded) in [
        ("##", "#"),
        ("###", "##"),
        ("####", "##"),
        ("#####", "###"),
        ("######", "####"),
    ] {
        let source = format!("env = VALUE,{raw}");
        let report = legacy_session::inspect(&source).unwrap();
        assert_eq!(
            report.draft.entries,
            [Entry::Environment {
                name: "VALUE".into(),
                value: decoded.into(),
                dbus: None
            }]
        );
    }
    for value in [
        "",
        "café",
        "#",
        "##",
        "###",
        "a#x##y###z",
        "commas,a,b",
        "quote\" and 'single'",
        "a\\b",
        "{{literal}}",
        "#\\{x}",
    ] {
        let entry = Entry::Environment {
            name: "VALUE".into(),
            value: value.into(),
            dbus: Some(true),
        };
        let encoded = legacy_session::encode(&entry).unwrap();
        let parsed = legacy_session::inspect(&encoded).unwrap();
        assert_eq!(parsed.draft.entries, [entry], "{encoded}");
    }
    let permission = Entry::Permission {
        binary: r"^/usr/bin/a\+b,part$".into(),
        permission: PermissionType::Screencopy,
        mode: PermissionMode::Deny,
    };
    let encoded = legacy_session::encode(&permission).unwrap();
    assert_eq!(
        legacy_session::inspect(&encoded).unwrap().draft.entries,
        [permission]
    );
    let source = "env = VALUE,old# keep comment\n";
    let target = Draft {
        entries: vec![Entry::Environment {
            name: "VALUE".into(),
            value: "##".into(),
            dbus: None,
        }],
    };
    let changed = legacy_session::edited_copy(source, &target).unwrap();
    assert!(changed.contains("# keep comment"));
    assert_eq!(legacy_session::inspect(&changed).unwrap().draft, target);
}
#[test]
fn unknown_expansion_scopes_continuations_and_conditionals_are_retained_without_execution() {
    let source="exec = known\nexec = $program\nenv = VALUE,{{1+2}}\nexec = continued \\\nenv = hidden,value\ngeneral {\n env = scoped,value\n}\n# hyprlang if something\nexec-once = conditional\n";
    let parsed = legacy_session::inspect(source).unwrap();
    assert_eq!(parsed.draft.entries.len(), 1);
    assert!(parsed.unresolved_count >= 4);
    let changed = legacy_session::edited_copy(source, &Draft::default()).unwrap();
    assert!(!changed.starts_with("exec = known"));
    for retained in [
        "exec = $program",
        "env = VALUE,{{1+2}}",
        "env = hidden,value",
        "env = scoped,value",
        "exec-once = conditional",
    ] {
        assert!(changed.contains(retained));
    }
    let before = std::env::var_os("HYE_LEGACY_SESSION_TEST");
    let marker = std::env::temp_dir().join(format!("hye-legacy-noexec-{}", std::process::id()));
    assert!(!marker.exists());
    let source = format!(
        "env = HYE_LEGACY_SESSION_TEST,not installed\nexec = touch {}\n",
        marker.display()
    );
    legacy_session::inspect(&source).unwrap();
    assert_eq!(std::env::var_os("HYE_LEGACY_SESSION_TEST"), before);
    assert!(!marker.exists());
}
#[test]
fn schema_v1_projects_remain_readable_but_cannot_claim_new_launch_modes() {
    let old = hyprland_editor_core::lua::source_hash(include_str!("../../../data/session-v1.json"));
    let basic = Draft {
        entries: vec![Entry::Command {
            command: "old command".into(),
            trigger: Trigger::Start,
        }],
    };
    assert!(session::accepts_schema(&old, &basic));
    let legacy = Draft {
        entries: vec![Entry::LegacyCommand {
            command: "app".into(),
            trigger: Trigger::Start,
            raw: false,
        }],
    };
    assert!(!session::accepts_schema(&old, &legacy));
    assert!(legacy.export_lua(BASELINE).is_err());
    assert!(legacy.validate(AUDITED_MAIN).is_err());
    let mut workspace =
        Workspace::new(hyprland_editor_core::Draft::new(BASELINE)).with_session(basic);
    workspace.session_schema_sha256 = Some(old);
    workspace.validate(&catalogue()).unwrap();
    workspace.session = legacy;
    assert!(workspace.validate(&catalogue()).is_err());
}
