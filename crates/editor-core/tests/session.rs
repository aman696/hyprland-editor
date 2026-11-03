use hyprland_editor_core::{
    catalogue,
    lua::{
        self,
        replay::{
            self, declaration_edit,
            project_copy::{self, Mode},
            Binding, Document, Profile, Program, Resolution,
        },
    },
    session::{Draft, Entry, PermissionMode, PermissionType, Trigger},
    workspace::Workspace,
    AUDITED_MAIN, BASELINE,
};
const SOURCE:&str="-- café\nhl.env('GTK_THEME','Nord',false);hl.exec_cmd('first application');hl.on('hyprland.start',function()hl.exec_cmd('startup application')end);hl.permission{binary='^/usr/bin/app$',type='screencopy',mode='ask'}\n";
fn draft(version: &str) -> Draft {
    let r = lua::parse(SOURCE, version, &catalogue()).unwrap();
    assert!(
        !r.opaque_effects && r.diagnostics.is_empty(),
        "{:?}",
        r.diagnostics
    );
    r.session
}
fn doc(id: &str, source: &str) -> Document {
    Document {
        id: id.into(),
        source: source.into(),
        sha256: lua::source_hash(source),
        load_name: format!("/config/{id}").into_bytes(),
    }
}
fn program(source: &str, version: &str) -> Program {
    Program {
        version: version.into(),
        profile: if cfg!(feature = "native55") {
            Profile::HyprlandLua55
        } else {
            Profile::HyprlandLua54
        },
        root: "root.lua".into(),
        documents: vec![doc("root.lua", source)],
        bindings: vec![],
        initial_package_path: Some(b"/config/?.lua".into()),
        initial_package_path_unknown_tail: false,
        working_directory: Some(b"/config".into()),
    }
}
#[test]
fn session_variants_roundtrip_with_order_optional_false_empty_values_and_schema_pins() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut d = draft(version);
        assert_eq!(d.entries.len(), 4);
        for trigger in Trigger::ALL {
            d.entries.push(Entry::Command {
                command: "printf 'not executed'".into(),
                trigger,
            });
        }
        for permission in PermissionType::ALL {
            for mode in PermissionMode::ALL {
                d.entries.push(Entry::Permission {
                    binary: "/usr/bin/.*".into(),
                    permission,
                    mode,
                });
            }
        }
        for dbus in [None, Some(false), Some(true)] {
            d.entries.push(Entry::Environment {
                name: "EMPTY_VALUE".into(),
                value: String::new(),
                dbus,
            });
        }
        let source = d.export_lua(version).unwrap();
        let parsed = lua::parse(&source, version, &catalogue()).unwrap();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.session, d);
        let workspace =
            Workspace::new(hyprland_editor_core::Draft::new(version)).with_session(d.clone());
        let bytes = workspace.to_json(&catalogue()).unwrap();
        let reopened =
            Workspace::from_json(std::str::from_utf8(&bytes).unwrap(), &catalogue()).unwrap();
        assert_eq!(reopened.session, d);
        let mut stale = workspace;
        stale.session_schema_sha256 = Some("0".repeat(64));
        assert!(stale.validate(&catalogue()).is_err());
    }
}
#[test]
fn import_and_replay_do_not_change_environment_or_run_commands_and_keep_callbacks_inert() {
    let marker = std::env::temp_dir().join(format!("hye-session-never-run-{}", std::process::id()));
    assert!(!marker.exists());
    let previous = std::env::var_os("HYE_SESSION_IMPORT_TEST");
    let source=format!("hl.env('HYE_SESSION_IMPORT_TEST','must not be installed',true);hl.exec_cmd({});hl.on('hyprland.start',function()os.execute('never')end);hl.permission{{binary='.*',type='plugin',mode='deny'}}",hyprland_editor_core::Value::String(format!("touch {}",marker.display())).lua());
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(&source, version);
        let report = replay::replay(&p).unwrap();
        assert!(report.halt.is_none(), "{:?}", report.halt);
        assert_eq!(report.events.len(), 4);
        let parsed = lua::parse(&source, version, &catalogue()).unwrap();
        assert!(parsed.opaque_effects);
        assert_eq!(parsed.session.entries.len(), 2);
    }
    assert!(!marker.exists());
    assert_eq!(std::env::var_os("HYE_SESSION_IMPORT_TEST"), previous);
}
#[test]
fn source_forms_edit_each_session_variant_in_a_module_and_combined_copy_includes_the_group() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p = program(
            "local value=require('part');hl.config{general={border_size=value}}",
            version,
        );
        p.documents
            .push(doc("part.lua", &format!("{SOURCE}return 2")));
        let request = match replay::replay(&p).unwrap().halt.unwrap() {
            replay::Halt::NeedsResolution { request } => request,
            other => panic!("{other:?}"),
        };
        p.bindings.push(Binding {
            request,
            resolution: Resolution::Loaded {
                document: "part.lua".into(),
                loader_data: vec![],
            },
        });
        let before = replay::replay(&p).unwrap();
        assert!(before.halt.is_none());
        for (api, replacement) in [
            ("hl.env", "hl.env('GTK_THEME','Changed',true)"),
            ("hl.exec_cmd", "hl.exec_cmd('new command')"),
            (
                "hl.on",
                "hl.on('hyprland.shutdown',function()hl.exec_cmd('new shutdown command')end)",
            ),
            (
                "hl.permission",
                "hl.permission{binary='^/usr/bin/other$',type='keyboard',mode='deny'}",
            ),
        ] {
            let event = before.events.iter().find(|e| e.api == api).unwrap();
            let selected = declaration_edit::inspect(&p, event.order, &catalogue()).unwrap();
            assert_eq!(selected.session.entries.len(), 1);
            let parsed = lua::parse(replacement, version, &catalogue()).unwrap();
            let target = Workspace::new(parsed.draft).with_session(parsed.session);
            let compared = declaration_edit::compare(&p, event.order, &target, &catalogue())
                .unwrap_or_else(|e| panic!("{api}: {e}"));
            assert!(compared.review.before_model_complete && compared.review.after_model_complete);
            assert_eq!(compared.review.revisions.len(), 1);
            assert!(compared.review.revisions[0].content.ends_with("return 2"));
        }
        let mut target = draft(version);
        if let Entry::Environment { value, .. } = &mut target.entries[0] {
            *value = "Combined".into();
        }
        let mut modes = [Mode::Keep; 9];
        modes[8] = Mode::ReplaceReached;
        let combined = project_copy::prepare(
            &p,
            &Workspace::new(hyprland_editor_core::Draft::new(version)).with_session(target),
            modes,
            &catalogue(),
        )
        .unwrap();
        assert_eq!(combined.removed_sites, 4);
        assert_eq!(combined.inserted_sites, 4);
        assert!(combined.files[0].content.contains("Combined"));
        assert!(combined.files[0]
            .content
            .contains("hl.config{general={border_size=value}}"));
        assert!(!combined.files[1].content.contains("hl.env"));
        assert!(combined.files[1].content.ends_with("return 2"));
    }
}
#[test]
fn unsupported_session_shapes_and_unsafe_names_remain_unresolved_instead_of_partial_edits() {
    for version in [BASELINE, AUDITED_MAIN] {
        for source in [
            "hl.env(\"BAD'NAME\",'x',true)",
            "hl.env('VALUE',os.getenv('HOME'))",
            "hl.exec_cmd('app',{float=true})",
            "hl.on('hyprland.start',named_callback)",
            "hl.on('hyprland.start',function()hl.exec_cmd('a');hl.exec_cmd('b')end)",
            "hl.permission{binary='(',type='plugin',mode='allow'}",
            "hl.permission{binary='.*',target='other',type='plugin',mode='allow'}",
        ] {
            let r = lua::parse(source, version, &catalogue()).unwrap();
            assert!(r.opaque_effects && !r.diagnostics.is_empty(), "{source}");
            assert!(r.session.is_empty(), "{source}");
        }
    }
    assert!(draft(BASELINE).export_lua("future").is_err());
}

#[test]
fn combined_session_replacement_cannot_silently_drop_unrepresented_event_callbacks() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = program(
            "hl.env('GTK_THEME','Nord');hl.on('window.open',function(window)window:close()end)",
            version,
        );
        let target = Workspace::new(hyprland_editor_core::Draft::new(version));
        let mut modes = [Mode::Keep; 9];
        modes[8] = Mode::ReplaceReached;
        assert!(project_copy::prepare(&p, &target, modes, &catalogue())
            .unwrap_err()
            .contains("cannot represent"));
        modes[8] = Mode::Keep;
        let kept = project_copy::prepare(&p, &target, modes, &catalogue()).unwrap();
        assert_eq!(kept.files[0].content, p.documents[0].source);
    }
}
