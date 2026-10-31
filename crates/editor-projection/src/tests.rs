use super::*;
use editor_core::{
    animation, binding, device, rule,
    source::{RecordedOwnership, SourceIdentity},
    workspace_rule, Draft, AUDITED_MAIN, BASELINE,
};
fn workspace(version: &str) -> Workspace {
    Workspace::new(Draft::new(version))
}
fn action(version: &str, name: &str, argument: &str) -> action::Action {
    let mut action = action::Action::new(action::find(name, version).unwrap());
    action
        .fields
        .insert("argument".into(), Value::String(argument.into()));
    action
}
fn bind(action: action::Action) -> binding::Draft {
    binding::Draft {
        legacy: None,
        binds: vec![binding::Bind {
            id: "bind".into(),
            chord: binding::Chord::parse("SUPER+Q"),
            actions: vec![action],
            flags: Default::default(),
            description: None,
            description_alias: false,
            device: None,
            submap: None,
        }],
        submaps: vec![],
        order: vec!["bind".into()],
    }
}
#[test]
fn opaque_source_hooks_actions_and_asset_paths_never_enter_generated_lua() {
    for version in [BASELINE, AUDITED_MAIN] {
        let source = "os.execute('HYE_RAW_SOURCE_COMMAND')\nreturn unknown_module()\n";
        let identity = SourceIdentity {
            selected_path: b"/unreachable/HYE_SOURCE_PATH.lua".to_vec(),
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
            sha256: editor_core::lua::source_hash(source),
            recorded_ownership: RecordedOwnership::UserOwned,
        };
        let mut workspace = workspace(version)
            .with_root(source.into(), identity)
            .with_bindings(bind(action(version, "exec_cmd", "HYE_BIND_COMMAND")));
        workspace.draft.values.insert(
            "decoration:screen_shader".into(),
            Value::String("/HYE_SHADER_PATH".into()),
        );
        workspace.draft.values.insert(
            "input:kb_file".into(),
            Value::String("/HYE_KEYMAP_PATH".into()),
        );
        let mut rule = workspace_rule::Rule::new("workspace");
        rule.fields.insert(
            "on_created_empty".into(),
            Value::String("HYE_EMPTY_HOOK".into()),
        );
        workspace = workspace.with_workspace_rules(vec![rule]);
        let mut gesture = gesture::Spec::new("gesture");
        gesture.action = gesture::GestureAction::Callbacks(gesture::Callbacks {
            start: None,
            update: None,
            finish: Some(gesture::Workflow {
                actions: vec![action(version, "exec_raw", "HYE_GESTURE_COMMAND")],
                event_parameter: Some("payload".into()),
            }),
        });
        workspace = workspace.with_gestures(gesture::Draft {
            declarations: vec![gesture],
        });
        let result = compile(&workspace, &Scenario::default()).unwrap();
        for marker in [
            "HYE_RAW_SOURCE_COMMAND",
            "HYE_SOURCE_PATH",
            "HYE_BIND_COMMAND",
            "HYE_SHADER_PATH",
            "HYE_KEYMAP_PATH",
            "HYE_EMPTY_HOOK",
            "HYE_GESTURE_COMMAND",
            "exec_cmd",
            "exec_raw",
            "on_created_empty",
        ] {
            assert!(!result.lua().contains(marker), "{marker}");
            if marker.starts_with("HYE_") {
                assert!(
                    !serde_json::to_string(result.report())
                        .unwrap()
                        .contains(marker),
                    "report leaked payload {marker}"
                );
            }
        }
        assert!(result.lua().contains("hl.dsp.no_op()"));
        assert_eq!(
            result
                .report()
                .entries
                .iter()
                .filter(|e| e.fidelity == Fidelity::MockedEffect)
                .count(),
            2
        );
        assert!(result
            .report()
            .entries
            .iter()
            .any(|e| e.subject == "source/root"));
        let mut changed = workspace.clone();
        changed.bindings.binds[0].actions[0].fields.insert(
            "argument".into(),
            Value::String("different omitted command".into()),
        );
        let next = compile(&changed, &Scenario::default()).unwrap();
        assert_eq!(result.lua(), next.lua());
        assert_ne!(result.identity(), next.identity());
    }
}
#[test]
fn every_family_projects_in_order_with_explicit_synthetic_identity_and_fidelity() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut workspace = workspace(version);
        let catalogue = editor_core::catalogue();
        let mut families = BTreeSet::new();
        for setting in catalogue.iter().filter(|s| s.version == version) {
            if families.contains(&setting.kind) {
                continue;
            }
            let value = match setting.kind.as_str() {
                "Bool" => Value::Bool(true),
                "Int" => Value::Int(1),
                "Float" => Value::Float(1.),
                "String" => Value::String("sample\"; assert(false); --\n".into()),
                "Color" => Value::Color(0xff112233),
                "Gradient" => Value::Gradient {
                    colors: vec![0xff112233, 0xff998877],
                    angle: 0.4,
                },
                "Vec2" => Value::Vec2([1., 2.]),
                "CssGap" => Value::CssGap([1, 2, 3, 4]),
                "FontWeight" => Value::FontWeight(400),
                _ => continue,
            };
            if setting.validate(&value).is_ok() {
                families.insert(setting.kind.clone());
                workspace.draft.values.insert(setting.id.clone(), value);
            }
        }
        assert_eq!(families.len(), 9);
        let mut monitor = monitor::Rule::new("monitor");
        monitor.output = monitor::Output::Connector("HYE_HOST_OUTPUT".into());
        monitor
            .fields
            .insert("mode".into(), Value::String("1920x1080@144".into()));
        monitor
            .fields
            .insert("icc".into(), Value::String("/HYE_ICC_PATH".into()));
        let mut device = device::Spec::new("device", "HYE_HOST_DEVICE");
        device
            .fields
            .insert("kb_file".into(), Value::String("/HYE_DEVICE_KEYMAP".into()));
        let mut window = rule::Rule::new("window", rule::Kind::Window);
        window.predicates.push(rule::Predicate {
            property: "class".into(),
            value: Value::String("sample".into()),
        });
        let mut layer = rule::Rule::new("layer", rule::Kind::Layer);
        layer.predicates.push(rule::Predicate {
            property: "namespace".into(),
            value: Value::String("sample-panel".into()),
        });
        let animations = animation::Draft {
            declarations: vec![
                animation::Declaration {
                    id: "curve".into(),
                    operation: animation::Operation::Curve(animation::Curve::bezier("samplecurve")),
                },
                animation::Declaration {
                    id: "node".into(),
                    operation: animation::Operation::Node(animation::Node::new("windows")),
                },
            ],
        };
        workspace = workspace
            .with_monitors(vec![monitor])
            .with_devices(vec![device])
            .with_workspace_rules(vec![workspace_rule::Rule::new("workspace")])
            .with_rules(vec![window, layer])
            .with_animations(animations)
            .with_bindings(bind(action(version, "exec_cmd", "unused")))
            .with_gestures(gesture::Draft {
                declarations: vec![gesture::Spec::new("gesture")],
            });
        let mut scenario = Scenario::default();
        scenario
            .monitor_targets
            .insert("monitor".into(), "HEADLESS-1".into());
        scenario
            .device_targets
            .insert("device".into(), "hye-lab-keyboard".into());
        let projection = compile(&workspace, &scenario).unwrap();
        for api in [
            "hl.config(",
            "hl.monitor(",
            "hl.device(",
            "hl.workspace_rule(",
            "hl.window_rule(",
            "hl.layer_rule(",
            "hl.curve(",
            "hl.animation(",
            "hl.bind(",
            "hl.gesture(",
        ] {
            assert!(projection.lua().contains(api), "{api}");
        }
        for host in [
            "HYE_HOST_OUTPUT",
            "HYE_HOST_DEVICE",
            "HYE_ICC_PATH",
            "HYE_DEVICE_KEYMAP",
            "1920x1080@144",
        ] {
            assert!(!projection.lua().contains(host), "{host}");
        }
        let first = projection.lua().find("hl.curve(").unwrap();
        let second = projection.lua().find("hl.animation(").unwrap();
        assert!(first < second);
        assert_eq!(projection.report().schema_pins.len(), 15);
        assert_eq!(
            projection.report().schema_pins["legacy-workspace"],
            editor_core::legacy_workspace::schema_hash()
        );
        assert_eq!(
            projection.identity(),
            compile(&workspace, &scenario).unwrap().identity()
        );
        assert!(projection
            .report()
            .entries
            .iter()
            .any(|e| e.subject == "monitor/monitor/mode"
                && e.fidelity == Fidelity::HardwareUnverified));
    }
}
#[test]
fn stale_mapping_invalid_pins_and_oversized_geometry_never_yield_a_projection() {
    let workspace = workspace(BASELINE);
    let mut scenario = Scenario::default();
    scenario
        .monitor_targets
        .insert("missing".into(), "HEADLESS-1".into());
    assert!(compile(&workspace, &scenario).is_err());
    let mut scenario = Scenario::default();
    scenario.outputs[0].name = "/proc/self/fd/3".into();
    assert!(compile(&workspace, &scenario).is_err());
    let mut scenario = Scenario::default();
    scenario.outputs[0].height = 1280;
    assert!(compile(&workspace, &scenario).is_err());
    let mut scenario = Scenario::default();
    scenario.outputs.push(scenario.outputs[0].clone());
    assert!(compile(&workspace, &scenario).is_err());
    let mut workspace = workspace.clone();
    workspace.schema.catalogue_sha256 = "0".repeat(64);
    assert!(compile(&workspace, &Scenario::default()).is_err());
}
#[test]
fn unmapped_hardware_is_explicit_and_inert_without_host_name_fallback() {
    let workspace = workspace(BASELINE)
        .with_monitors(vec![monitor::Rule::new("m")])
        .with_devices(vec![device::Spec::new("d", "host-keyboard")]);
    let result = compile(&workspace, &Scenario::default()).unwrap();
    assert!(!result.lua().contains("hl.device("));
    for subject in ["monitor/m", "device/d"] {
        assert!(result
            .report()
            .entries
            .iter()
            .any(|e| e.subject == subject && e.fidelity == Fidelity::Unsupported));
    }
    let mut scenario = Scenario::default();
    scenario
        .device_targets
        .insert("d".into(), "host-keyboard".into());
    assert!(compile(&workspace, &scenario).is_err());
}

#[test]
fn every_audited_action_constructor_becomes_an_identified_inert_mock() {
    for version in [BASELINE, AUDITED_MAIN] {
        let definitions = action::catalogue(version);
        assert!(definitions.len() >= 51);
        for definition in definitions {
            let mut original = action::Action::new(&definition);
            for (key, value) in &mut original.fields {
                if matches!(value, Value::Bool(_)) {
                    *value = Value::Bool(true);
                }
                if definition.name == "workspace.change_id" && key == "id" {
                    *value = Value::Int(1);
                }
            }
            original.at_load = true;
            original
                .validate(version)
                .unwrap_or_else(|e| panic!("{}: {e}", definition.name));
            let workspace = workspace(version).with_bindings(bind(original));
            let result = compile(&workspace, &Scenario::default()).unwrap();
            assert!(result.lua().contains("hl.dsp.no_op()"));
            for other in action::catalogue(version)
                .iter()
                .filter(|a| a.name != "no_op")
            {
                assert!(!result.lua().contains(&format!("hl.dsp.{}(", other.name)));
            }
            assert!(result
                .report()
                .entries
                .iter()
                .any(|e| e.mocked_intent.as_deref() == Some(definition.name.as_str())));
        }
    }
}
#[test]
fn projection_budget_refuses_a_large_valid_draft_without_returning_truncated_output() {
    let original = action::Action::new(action::find("no_op", BASELINE).unwrap());
    let mut bindings = binding::Draft::default();
    for index in 0..258 {
        let mut next = bind(original.clone()).binds.remove(0);
        next.id = format!("b{index}");
        next.actions = vec![original.clone(); 64];
        bindings.order.push(next.id.clone());
        bindings.binds.push(next);
    }
    let workspace = workspace(BASELINE).with_bindings(bindings);
    workspace.validate(&editor_core::catalogue()).unwrap();
    assert!(compile(&workspace, &Scenario::default())
        .unwrap_err()
        .contains("entry budget"));
}

#[test]
fn session_commands_environment_and_permission_payloads_are_omitted_and_reported_without_disclosure(
) {
    use editor_core::session::{Draft as Session, Entry, PermissionMode, PermissionType, Trigger};
    for version in [BASELINE, AUDITED_MAIN] {
        let workspace = workspace(version).with_session(Session {
            entries: vec![
                Entry::Environment {
                    name: "PRIVATE_SECRET_NAME".into(),
                    value: "PRIVATE_SECRET_VALUE".into(),
                    dbus: Some(true),
                },
                Entry::Command {
                    command: "PRIVATE_COMMAND_VALUE".into(),
                    trigger: Trigger::Start,
                },
                Entry::Permission {
                    binary: "PRIVATE_BINARY_PATTERN".into(),
                    permission: PermissionType::Plugin,
                    mode: PermissionMode::Allow,
                },
            ],
        });
        let projection = compile(&workspace, &Scenario::default()).unwrap();
        let encoded = format!(
            "{}{}",
            projection.lua(),
            serde_json::to_string(projection.report()).unwrap()
        );
        for private in [
            "PRIVATE_SECRET_NAME",
            "PRIVATE_SECRET_VALUE",
            "PRIVATE_COMMAND_VALUE",
            "PRIVATE_BINARY_PATTERN",
        ] {
            assert!(!encoded.contains(private));
        }
        assert_eq!(
            projection
                .report
                .entries
                .iter()
                .filter(|e| e.subject.starts_with("session/")
                    && e.fidelity == Fidelity::OmittedUntrusted)
                .count(),
            3
        );
        assert!(!projection.lua.contains("hl.env"));
        assert!(!projection.lua.contains("hl.exec_cmd"));
        assert!(!projection.lua.contains("hl.permission"));
    }
}
#[test]
fn legacy_monitor_operations_never_become_implicitly_converted_preview_commands() {
    use editor_core::{legacy_monitor, workspace::SourceLanguage};
    let source = "monitor = PRIVATE_OUTPUT,transform,1\nmonitorv2 {\n output = PRIVATE_OUTPUT_BLOCK\n supports_hdr = 1\n}\n";
    let hash = editor_core::lua::source_hash(source);
    let identity = SourceIdentity {
        selected_path: b"/private/config.conf".to_vec(),
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
    let mut workspace = workspace(BASELINE)
        .with_root(source.into(), identity)
        .with_monitors(legacy_monitor::inspect(source).unwrap().rules);
    workspace.sources[0].language = SourceLanguage::Legacy;
    workspace.legacy_monitor_origin_sha256 = Some(hash);
    workspace.legacy_monitor_schema_sha256 = Some(legacy_monitor::schema_hash());
    let projection = compile(&workspace, &Scenario::default()).unwrap();
    assert!(!projection.lua().contains("PRIVATE_OUTPUT"));
    assert!(!serde_json::to_string(projection.report())
        .unwrap()
        .contains("PRIVATE_OUTPUT"));
    assert!(projection
        .report()
        .entries
        .iter()
        .any(|entry| entry.subject == "monitor/legacy-monitor-0"
            && entry.fidelity == Fidelity::Unsupported));
}

#[test]
fn legacy_bindings_are_omitted_without_command_description_device_or_submap_payloads() {
    use editor_core::{legacy_binding, workspace::SourceLanguage};
    let source = "$device = PRIVATE_DEVICE\n$description = PRIVATE_DESCRIPTION\n$command = PRIVATE_COMMAND\n$map = PRIVATE_SUBMAP\nbinddk = SUPER,Q,$device,$description,exec,$command\nsubmap = $map\nbind = ,catchall,submap,reset\nsubmap = reset\nunbind = all\n";
    let hash = editor_core::lua::source_hash(source);
    let identity = SourceIdentity {
        selected_path: b"/private/config.conf".to_vec(),
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
    let mut workspace = workspace(BASELINE)
        .with_root(source.into(), identity)
        .with_bindings(binding::Draft {
            legacy: Some(legacy_binding::inspect(source).unwrap().draft),
            ..Default::default()
        });
    workspace.sources[0].language = SourceLanguage::Legacy;
    workspace.legacy_binding_origin_sha256 = Some(hash);
    workspace.legacy_binding_schema_sha256 = Some(legacy_binding::schema_hash());
    let projection = compile(&workspace, &Scenario::default()).unwrap();
    let report = serde_json::to_string(projection.report()).unwrap();
    for payload in [
        "PRIVATE_DEVICE",
        "PRIVATE_DESCRIPTION",
        "PRIVATE_COMMAND",
        "PRIVATE_SUBMAP",
    ] {
        assert!(!projection.lua().contains(payload));
        assert!(!report.contains(payload));
    }
    assert_eq!(
        projection
            .report()
            .entries
            .iter()
            .filter(|entry| entry.subject.starts_with("legacy-binding/")
                && entry.fidelity == Fidelity::Unsupported)
            .count(),
        5
    );
}

#[test]
fn native_workspace_preview_omits_commands_names_outputs_and_custom_option_payloads() {
    use editor_core::{legacy_workspace, workspace::SourceLanguage};
    let source = "workspace = name:PRIVATE_NAME,monitor:PRIVATE_OUTPUT,on-created-empty:PRIVATE_COMMAND,layoutopt:PRIVATE_KEY:PRIVATE_VALUE\n";
    let hash = editor_core::lua::source_hash(source);
    let identity = SourceIdentity {
        selected_path: b"/private/config.conf".to_vec(),
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
    let mut workspace = workspace(BASELINE)
        .with_root(source.into(), identity)
        .with_workspace_rules(legacy_workspace::inspect(source).unwrap().rules);
    workspace.sources[0].language = SourceLanguage::Legacy;
    workspace.legacy_workspace_origin_sha256 = Some(hash);
    workspace.legacy_workspace_schema_sha256 = Some(legacy_workspace::schema_hash());
    let projection = compile(&workspace, &Scenario::default()).unwrap();
    for payload in [
        "PRIVATE_NAME",
        "PRIVATE_OUTPUT",
        "PRIVATE_COMMAND",
        "PRIVATE_KEY",
        "PRIVATE_VALUE",
    ] {
        assert!(!projection.lua().contains(payload));
        assert!(!serde_json::to_string(projection.report())
            .unwrap()
            .contains(payload));
    }
    assert!(!serde_json::to_string(projection.report())
        .unwrap()
        .contains("PRIVATE_OUTPUT"));
    assert!(projection
        .report()
        .entries
        .iter()
        .any(|entry| entry.subject == "workspace/legacy-workspace-0"
            && entry.fidelity == Fidelity::OmittedUntrusted));
}
