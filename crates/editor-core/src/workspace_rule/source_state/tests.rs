use super::*;
fn input(selector: &[u8]) -> Declaration {
    Declaration {
        selector: selector.into(),
        enabled: true,
        fields: BTreeMap::new(),
        layout_options: BTreeMap::new(),
    }
}
fn state(version: &str) -> State {
    State::new(version, Limits::default()).unwrap()
}
fn records(state: &State) -> Vec<RuleRecord> {
    state.records().cloned().collect()
}
fn reference(r: &Reference) -> Argument {
    Argument::Reference(r.clone())
}
fn enabled(state: &mut State, r: &Reference) -> CallResult {
    state.call(Method::IsEnabled, &[reference(r)]).unwrap()
}
fn set(state: &mut State, r: &Reference, value: bool) {
    assert_eq!(
        state
            .call(Method::SetEnabled, &[reference(r), Argument::Bool(value)])
            .unwrap(),
        CallResult::Nothing
    );
}
fn retire_fixture(state: &mut State, r: &Reference) {
    let id = state.rule_id(r).unwrap() as usize;
    if let Some(old) = state.rules[id].take() {
        state.retained -= cost(&old, state.limits.retained_bytes).unwrap();
    }
}

#[test]
fn literal_merge_keeps_shared_rule_identity_and_native_optional_distinctions() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut s = state(version);
        let mut first = input(b"1");
        first.fields.insert(
            "monitor".into(),
            FieldValue::Known(NativeValue::Bytes(b"DP-1".to_vec())),
        );
        first.fields.insert(
            "persistent".into(),
            FieldValue::Known(NativeValue::Bool(true)),
        );
        first.fields.insert(
            "float_gaps".into(),
            FieldValue::Known(NativeValue::CssGap([5; 4])),
        );
        first
            .fields
            .insert("border_size".into(), FieldValue::Known(NativeValue::Int(7)));
        first.fields.insert(
            "on_created_empty".into(),
            FieldValue::Known(NativeValue::Bytes(b"inert command".to_vec())),
        );
        first
            .layout_options
            .insert(b"a".to_vec(), OptionValue::Known(b"true".to_vec()));
        let a = s.declare(first).unwrap();
        let mut second = input(b"1");
        second.enabled = false;
        for (name, value) in [
            ("monitor", NativeValue::Bytes(vec![])),
            ("persistent", NativeValue::Bool(false)),
            ("float_gaps", NativeValue::CssGap([0; 4])),
            ("border_size", NativeValue::Int(0)),
            ("on_created_empty", NativeValue::Bytes(vec![])),
        ] {
            second.fields.insert(name.into(), FieldValue::Known(value));
        }
        second
            .layout_options
            .insert(b"b".to_vec(), OptionValue::Known(b"false".to_vec()));
        let b = s.declare(second).unwrap();
        assert!(!s.raw_equal(&a, &b).unwrap());
        assert!(s.equal(&a, &b).unwrap());
        assert_eq!(s.records().count(), 1);
        assert_eq!(enabled(&mut s, &b), CallResult::Bool(true));
        let rule = s.records().next().unwrap();
        assert_eq!(
            rule.monitor,
            FieldValue::Known(NativeValue::Bytes(b"DP-1".to_vec()))
        );
        assert_eq!(
            rule.fields["persistent"],
            FieldValue::Known(NativeValue::Bool(false))
        );
        assert_eq!(
            rule.fields["float_gaps"],
            FieldValue::Known(NativeValue::CssGap([0; 4]))
        );
        assert_eq!(
            rule.fields["border_size"],
            FieldValue::Known(NativeValue::Int(0))
        );
        assert_eq!(
            rule.fields["on_created_empty"],
            FieldValue::Known(NativeValue::Bytes(vec![]))
        );
        assert_eq!(rule.layout_options.len(), 2);
        assert!(!rule.fields.contains_key("no_border"));
        assert_eq!(s.refresh_requests(), 2);
    }
}

#[test]
fn method_mutations_change_which_enabled_literal_record_receives_later_declarations() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut s = state(version);
        let a = s.declare(input(b"1")).unwrap();
        let alias = s.wrap(&a).unwrap();
        set(&mut s, &alias, false);
        assert_eq!(enabled(&mut s, &a), CallResult::Bool(false));
        let b = s.declare(input(b"1")).unwrap();
        assert!(!s.equal(&a, &b).unwrap());
        let mut declaration = input(b"1");
        declaration.enabled = false;
        declaration
            .fields
            .insert("default".into(), FieldValue::Known(NativeValue::Bool(true)));
        let c = s.declare(declaration).unwrap();
        assert!(s.equal(&b, &c).unwrap());
        assert_eq!(s.records().count(), 2);
        assert_eq!(enabled(&mut s, &c), CallResult::Bool(true));
        set(&mut s, &a, true);
        let d = s.declare(input(b"1")).unwrap();
        assert!(
            s.equal(&a, &d).unwrap(),
            "The first enabled exact selector wins again"
        );
        assert!(!s.equal(&b, &d).unwrap());
    }
}

#[test]
fn uncertainty_is_retained_until_an_unambiguous_later_override() {
    let mut s = state(BASELINE);
    let mut d = input(b"name:a");
    d.fields
        .insert("monitor".into(), FieldValue::Unresolved { cause: 1 });
    d.fields
        .insert("no_border".into(), FieldValue::Unresolved { cause: 2 });
    d.layout_options
        .insert(b"custom".to_vec(), OptionValue::Unresolved { cause: 3 });
    let a = s.declare(d).unwrap();
    let mut d = input(b"name:a");
    d.fields.insert(
        "monitor".into(),
        FieldValue::Known(NativeValue::Bytes(vec![])),
    );
    let b = s.declare(d).unwrap();
    assert!(s.equal(&a, &b).unwrap());
    assert_eq!(
        s.records().next().unwrap().monitor,
        FieldValue::Unresolved { cause: 1 }
    );
    let mut d = input(b"name:a");
    d.fields.insert(
        "monitor".into(),
        FieldValue::Known(NativeValue::Bytes(b"DP-2".to_vec())),
    );
    d.fields.insert(
        "no_border".into(),
        FieldValue::Known(NativeValue::Bool(false)),
    );
    d.layout_options
        .insert(b"custom".to_vec(), OptionValue::Known(b"literal".to_vec()));
    s.declare(d).unwrap();
    let rule = s.records().next().unwrap();
    assert_eq!(
        rule.monitor,
        FieldValue::Known(NativeValue::Bytes(b"DP-2".to_vec()))
    );
    assert_eq!(
        rule.fields["no_border"],
        FieldValue::Known(NativeValue::Bool(false))
    );
    assert_eq!(
        rule.layout_options[b"custom".as_slice()],
        OptionValue::Known(b"literal".to_vec())
    );
}

#[test]
fn invalid_inputs_foreign_realms_and_bounds_refuse_without_changing_rule_state() {
    let mut s = state(BASELINE);
    let a = s.declare(input(b"1")).unwrap();
    let mut other = state(BASELINE);
    let foreign = other.declare(input(b"1")).unwrap();
    let before = records(&s);
    let bytes = s.retained_bytes();
    let refresh = s.refresh_requests();
    assert_eq!(
        s.call(
            Method::SetEnabled,
            &[reference(&foreign), Argument::Bool(false)]
        ),
        Err(Error::ForeignReference)
    );
    assert_eq!(s.wrap(&foreign).unwrap_err(), Error::ForeignReference);
    assert_eq!(s.equal(&a, &foreign), Err(Error::ForeignReference));
    assert_eq!(
        s.call(
            Method::SetEnabled,
            &[Argument::Other, Argument::Bool(false)]
        ),
        Err(Error::WrongReceiver)
    );
    assert_eq!(
        s.call(Method::SetEnabled, &[reference(&a), Argument::Other]),
        Err(Error::ExpectedBoolean)
    );
    for mut bad in [
        input(b"invalid\0selector"),
        input(b"2"),
        input(b"3"),
        input(b"4"),
    ] {
        if bad.selector == b"2" {
            bad.fields
                .insert("outside".into(), FieldValue::Known(NativeValue::Int(1)));
        }
        if bad.selector == b"3" {
            bad.fields.insert(
                "border_size".into(),
                FieldValue::Known(NativeValue::Bool(true)),
            );
        }
        if bad.selector == b"4" {
            bad.layout_options
                .insert(b"key\0tail".to_vec(), OptionValue::Known(vec![]));
        }
        assert_eq!(s.declare(bad).unwrap_err(), Error::InvalidDeclaration);
    }
    assert_eq!(records(&s), before);
    assert_eq!(s.retained_bytes(), bytes);
    assert_eq!(s.refresh_requests(), refresh);
    assert!(matches!(
        State::new("unsupported", Limits::default()),
        Err(Error::UnsupportedVersion)
    ));
}

#[test]
fn resource_refusal_and_boolean_size_accounting_are_atomic() {
    let mut s = State::new(
        BASELINE,
        Limits {
            rules: 1,
            wrappers: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let a = s.declare(input(b"1")).unwrap();
    let bytes = s.retained_bytes();
    set(&mut s, &a, false);
    assert_eq!(s.retained_bytes(), bytes + 1);
    set(&mut s, &a, true);
    assert_eq!(s.retained_bytes(), bytes);
    let before = records(&s);
    assert_eq!(s.declare(input(b"2")).unwrap_err(), Error::Budget);
    assert_eq!(records(&s), before);
    s.wrap(&a).unwrap();
    let bytes = s.retained_bytes();
    assert_eq!(s.declare(input(b"1")).unwrap_err(), Error::Budget);
    assert_eq!(records(&s), before);
    assert_eq!(s.retained_bytes(), bytes);
    let mut s = State::new(
        BASELINE,
        Limits {
            retained_bytes: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(s.declare(input(b"1")).unwrap_err(), Error::Budget);
    assert_eq!(s.records().count(), 0);
    let mut s = State::new(
        BASELINE,
        Limits {
            steps: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let a = s.declare(input(b"1")).unwrap();
    assert_eq!(
        s.call(Method::IsEnabled, &[reference(&a)]),
        Err(Error::Budget)
    );
}

#[test]
fn model_object_methods_match_each_native_reference_scenario() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../data/workspace-object-fixtures.json"
    ))
    .unwrap();
    assert_eq!(fixtures["cases"].as_array().unwrap().len(), 42);
    let mut compared = 0;
    for row in fixtures["cases"].as_array().unwrap() {
        let version = if row["version"] == "baseline" {
            BASELINE
        } else {
            AUDITED_MAIN
        };
        let mut s = state(version);
        let a = s.declare(input(b"a")).unwrap();
        let b = s.declare(input(b"b")).unwrap();
        let refresh = s.refresh_requests();
        let nil = || serde_json::json!({"type":"nil"});
        let boolean = |b| serde_json::json!({"type":"bool","value":b});
        let integer = |i| serde_json::json!({"type":"int","value":i});
        let text = |s: &str| serde_json::json!({"type":"bytes","value":s.as_bytes()});
        let name = row["case"].as_str().unwrap();
        let values = match name {
            "initial" => vec![
                text("userdata"),
                boolean(enabled(&mut s, &a) == CallResult::Bool(true)),
            ],
            "disable" => {
                set(&mut s, &a, false);
                vec![
                    integer(0),
                    boolean(enabled(&mut s, &a) == CallResult::Bool(true)),
                ]
            }
            "enable_already_enabled" => {
                set(&mut s, &a, true);
                vec![
                    integer(0),
                    boolean(enabled(&mut s, &a) == CallResult::Bool(true)),
                ]
            }
            "get_extra_argument" => {
                assert_eq!(
                    s.call(Method::IsEnabled, &[reference(&a), Argument::Other])
                        .unwrap(),
                    CallResult::Bool(true)
                );
                vec![boolean(true)]
            }
            "set_extra_argument" => {
                s.call(
                    Method::SetEnabled,
                    &[reference(&a), Argument::Bool(false), Argument::Other],
                )
                .unwrap();
                vec![boolean(enabled(&mut s, &a) == CallResult::Bool(true))]
            }
            "wrong_set_type" | "missing_set_argument" | "expired_wrong_set_type" => {
                if name == "expired_wrong_set_type" {
                    retire_fixture(&mut s, &a);
                }
                let mut args = vec![reference(&a)];
                if name != "missing_set_argument" {
                    args.push(Argument::Other);
                }
                assert_eq!(
                    s.call(Method::SetEnabled, &args),
                    Err(Error::ExpectedBoolean)
                );
                let mut out = vec![boolean(false), text("string")];
                if name != "expired_wrong_set_type" {
                    out.push(boolean(enabled(&mut s, &a) == CallResult::Bool(true)));
                }
                out
            }
            "wrong_self" | "dot_call_missing_self" => {
                assert_eq!(
                    s.call(
                        if name == "wrong_self" {
                            Method::IsEnabled
                        } else {
                            Method::SetEnabled
                        },
                        &[Argument::Other]
                    ),
                    Err(Error::WrongReceiver)
                );
                vec![boolean(false), text("string")]
            }
            "unknown_member" | "numeric_member" => {
                assert_eq!(
                    s.lookup(
                        &a,
                        if name == "unknown_member" {
                            b"outside"
                        } else {
                            b"3"
                        }
                    )
                    .unwrap(),
                    None
                );
                vec![text("nil")]
            }
            "bad_member_type" => {
                assert_eq!(
                    s.member(&a, MemberKey::WrongType),
                    Err(Error::ExpectedMemberString)
                );
                vec![boolean(false), text("string")]
            }
            "nul_method_alias" => {
                assert_eq!(
                    s.lookup(&a, b"set_enabled\0tail").unwrap(),
                    Some(Method::SetEnabled)
                );
                set(&mut s, &a, false);
                assert_eq!(
                    s.lookup(&a, b"is_enabled\0tail").unwrap(),
                    Some(Method::IsEnabled)
                );
                vec![boolean(false)]
            }
            "shared_rule_wrappers" => {
                let alias = s.wrap(&a).unwrap();
                set(&mut s, &a, false);
                vec![
                    boolean(s.raw_equal(&a, &alias).unwrap()),
                    boolean(s.equal(&a, &alias).unwrap()),
                    boolean(enabled(&mut s, &alias) == CallResult::Bool(true)),
                ]
            }
            "different_rules" => vec![boolean(s.equal(&a, &b).unwrap())],
            "unbound_method" => {
                set(&mut s, &b, false);
                vec![
                    boolean(enabled(&mut s, &a) == CallResult::Bool(true)),
                    boolean(enabled(&mut s, &b) == CallResult::Bool(true)),
                ]
            }
            "live_tostring" => {
                let display = s.display(&a).unwrap();
                assert!(matches!(display, DisplayValue::LiveAddressDependent { .. }));
                // Match the native fixture's prefix predicate without
                // fabricating the unknown pointer-dependent suffix.
                vec![boolean(
                    display.known_prefix().starts_with(b"HL.WorkspaceRule("),
                )]
            }
            "expired_get_and_set" => {
                retire_fixture(&mut s, &a);
                assert_eq!(enabled(&mut s, &a), CallResult::Nil);
                set(&mut s, &a, false);
                assert_eq!(s.display(&a).unwrap(), DisplayValue::Expired);
                vec![nil(), integer(0), text("HL.WorkspaceRule(expired)")]
            }
            "both_expired_equal" => {
                retire_fixture(&mut s, &a);
                retire_fixture(&mut s, &b);
                vec![boolean(s.equal(&a, &b).unwrap())]
            }
            "one_expired_different" => {
                retire_fixture(&mut s, &a);
                vec![boolean(s.equal(&a, &b).unwrap())]
            }
            other => panic!("Unreviewed native object case {other}"),
        };
        assert_eq!(
            serde_json::Value::Array(values),
            row["observation"]["returns"],
            "{version} {name}"
        );
        assert_eq!(
            serde_json::json!(s.refresh_requests() - refresh),
            row["observation"]["refresh_requests"],
            "{version} {name}"
        );
        let live = s
            .records()
            .map(|r| (String::from_utf8(r.selector.clone()).unwrap(), r.enabled))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            serde_json::to_value(live).unwrap(),
            row["observation"]["live_enabled"],
            "{version} {name}"
        );
        compared += 1;
    }
    assert_eq!(compared, 42);
}

#[test]
fn literal_selectors_and_consumed_byte_options_are_not_normalized_into_other_rules() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut s = state(version);
        let a = s.declare(input(b"1")).unwrap();
        let b = s.declare(input(b"01")).unwrap();
        assert!(!s.equal(&a, &b).unwrap());
        let mut raw = input(&[0xff, b'a']);
        raw.layout_options
            .insert(vec![0xff, b'k'], OptionValue::Known(vec![0xfe, b'v']));
        let c = s.declare(raw).unwrap();
        assert!(!s.equal(&a, &c).unwrap());
        assert_eq!(
            s.records().last().unwrap().layout_options[&vec![0xff, b'k']],
            OptionValue::Known(vec![0xfe, b'v'])
        );
        let invalid = Reference {
            realm: s.realm.clone(),
            wrapper: usize::MAX,
        };
        let before = records(&s);
        let bytes = s.retained_bytes();
        assert_eq!(
            s.call(
                Method::SetEnabled,
                &[reference(&invalid), Argument::Bool(false)]
            ),
            Err(Error::UnknownReference)
        );
        assert_eq!(records(&s), before);
        assert_eq!(s.retained_bytes(), bytes);
    }
}

#[test]
fn every_factory_accepts_post_parse_storage_shape_without_draft_policy_bounds() {
    for version in [BASELINE, AUDITED_MAIN] {
        for field in factories(version).unwrap() {
            let mut s = state(version);
            let mut d = input(b"1");
            let native = match field.kind.as_str() {
                "Bool" => NativeValue::Bool(false),
                "Int" => NativeValue::Int(i64::MIN),
                "String" => NativeValue::Bytes(vec![]),
                "CssGap" => NativeValue::CssGap([-1, 0, 2, 3]),
                _ => panic!("Unreviewed native factory"),
            };
            d.fields
                .insert(field.key.clone(), FieldValue::Known(native.clone()));
            s.declare(d).unwrap();
            let record = s.records().next().unwrap();
            if field.key == "monitor" {
                assert_eq!(record.monitor, FieldValue::Known(native));
                assert!(record.fields.is_empty());
            } else {
                assert_eq!(record.fields.len(), 1);
                assert_eq!(record.fields[&field.key], FieldValue::Known(native));
            }
        }
    }
}

#[test]
fn rule_object_effects_remain_unmodeled_after_argument_observation() {
    use crate::lua::replay::{self, Document, Halt, Profile, Program};
    for version in [BASELINE, AUDITED_MAIN] {
        let source =
            "local rule=hl.workspace_rule{workspace='1'};hl.config{general={border_size=7}}";
        let program = Program {
            version: version.into(),
            profile: Profile::HyprlandLua54,
            root: "root".into(),
            documents: vec![Document {
                id: "root".into(),
                source: source.into(),
                sha256: lua::source_hash(source),
                load_name: b"/inert-workspace-reference.lua".to_vec(),
            }],
            bindings: vec![],
            initial_package_path: Some(vec![]),
            initial_package_path_unknown_tail: false,
            working_directory: None,
        };
        let report = replay::replay(&program).unwrap();
        assert!(matches!(report.halt, Some(Halt::Unresolved { .. })));
        assert_eq!(report.events.len(), 1);
        assert_eq!(report.events[0].api, "hl.workspace_rule");
        assert!(report.returns.is_empty());
        assert!(!report.runtime_verified);
    }
}
