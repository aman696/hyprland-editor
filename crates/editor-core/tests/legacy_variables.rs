use hyprland_editor_core::{
    binding, catalogue, legacy, legacy_binding, legacy_variables as codec,
    source::{RecordedOwnership, SourceIdentity},
    workspace::{SourceLanguage, Workspace},
    Value,
};
const SOURCE:&str="# keep café\r\n$mainMod = SUPER # modifier\r\n$terminal = old application # command\r\n$color = rgba(aabbccdd) # color\r\n$alias = $terminal # reference\r\n$terminal = second app # reassignment\r\nbind = $mainMod,Q,exec,$terminal # first\r\nbind = $mainMod,W,exec,$alias # alias use\r\nsource = missing.conf\r\n";
fn workspace(source: &str) -> Workspace {
    let c = catalogue();
    let d = legacy::Document::new(source.into(), &c).unwrap();
    let hash = hyprland_editor_core::lua::source_hash(source);
    let origin = SourceIdentity {
        selected_path: b"/never-read/variables.conf".to_vec(),
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
    let mut w = Workspace::new(d.report.draft)
        .with_root(source.into(), origin)
        .with_bindings(binding::Draft {
            legacy: Some(d.bindings.draft),
            ..Default::default()
        });
    w.sources[0].language = SourceLanguage::Legacy;
    w.legacy_binding_origin_sha256 = Some(hash);
    w.legacy_binding_schema_sha256 = Some(legacy_binding::schema_hash());
    w
}
#[test]
fn ordered_definition_templates_preserve_aliases_duplicates_hashes_and_reference_boundaries() {
    let r = codec::inspect(SOURCE).unwrap();
    assert_eq!(r.definitions.len(), 5);
    assert_eq!(r.definitions[3].value, "$terminal");
    assert_eq!(codec::edited_copy(SOURCE, &r.definitions).unwrap(), SOURCE);
    let mut target = r.definitions;
    target[0].value = "ALT".into();
    target[1].value = "path \\{literal\\} # ## ### $unknown {{unknown + 2}}".into();
    let out = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(out.starts_with("# keep café\r\n$mainMod = ALT # modifier\r\n"));
    assert!(out.contains("# reference\r\n"));
    assert_eq!(codec::inspect(&out).unwrap().definitions, target);
    assert!(out.ends_with("source = missing.conf\r\n"));
    assert_eq!(
        codec::references("$a = value\n$a2 = $a # $a\nbind = $a,Q,exec,$a2\n", "a").unwrap(),
        vec![2, 3]
    );
}
#[test]
fn definition_add_duplicate_remove_reorder_and_rename_keep_untouched_source() {
    let r = codec::inspect(SOURCE).unwrap();
    let mut target = r.definitions.clone();
    target.swap(0, 1);
    target[0].name = "renamed".into();
    let mut copy = target[0].clone();
    copy.id = "extra".into();
    copy.name = "new".into();
    copy.value = "0xffaabbcc".into();
    target.push(copy);
    let out = codec::edited_copy(SOURCE, &target).unwrap();
    assert!(out.contains("$new = 0xffaabbcc\r\nbind ="));
    assert!(out.contains("$mainMod,Q,exec,$terminal"));
    assert!(out.contains("# modifier"));
    assert!(out.contains("# command"));
    target.remove(1);
    let out = codec::edited_copy(SOURCE, &target).unwrap();
    assert_eq!(
        codec::inspect(&out)
            .unwrap()
            .definitions
            .iter()
            .map(|d| (&d.name, &d.value))
            .collect::<Vec<_>>(),
        target
            .iter()
            .map(|d| (&d.name, &d.value))
            .collect::<Vec<_>>()
    );
    let empty = codec::edited_copy(SOURCE, &[]).unwrap();
    assert!(!empty.contains("$mainMod ="));
    assert!(empty.contains("# modifier"));
    assert!(empty.contains("bind = $mainMod"));
    let new = codec::edited_copy(
        "# start\nsource = other.conf\n",
        &[codec::Definition {
            id: "new".into(),
            name: "before_include".into(),
            value: "yes".into(),
        }],
    )
    .unwrap();
    assert!(new.starts_with("$before_include = yes\n# start\n"));
}
#[test]
fn variable_proposals_include_independent_edits_and_rebuild_only_supported_source_values() {
    let source = format!("{SOURCE}general:border_size = 2\n");
    let mut w = workspace(&source);
    w.draft
        .values
        .insert("general:border_size".into(), Value::Int(4));
    let legacy_binding::Entry::Bind { argument, .. } =
        &mut w.bindings.legacy.as_mut().unwrap().entries[0]
    else {
        panic!()
    };
    *argument = "explicit private command".into();
    let mut target = codec::inspect(&source).unwrap().definitions;
    target[0].value = "CTRL SHIFT".into();
    target[1].value = "new initial command".into();
    target[4].value = "new reassigned command".into();
    let output = codec::prepare(&w, &target, &catalogue()).unwrap();
    let r = legacy::Document::new(output.clone(), &catalogue()).unwrap();
    assert_eq!(r.report.draft.values["general:border_size"], Value::Int(4));
    assert!(output.contains("bind = $mainMod,Q,exec,explicit private command # first"));
    assert!(output.contains("bind = $mainMod,W,exec,$alias # alias use"));
    let legacy_binding::Entry::Bind {
        chord, argument, ..
    } = &r.bindings.draft.entries[0]
    else {
        panic!()
    };
    assert_eq!(chord.modifiers.len(), 2);
    assert!(chord.modifiers.contains(&"CTRL".into()) && chord.modifiers.contains(&"SHIFT".into()));
    assert_eq!(argument, "explicit private command");
    let legacy_binding::Entry::Bind { argument, .. } = &r.bindings.draft.entries[1] else {
        panic!()
    };
    assert_eq!(argument, "new initial command");
    assert_eq!(w.root().unwrap().content, source);
    assert!(w.legacy_revision.is_none());
}
#[test]
fn private_native_revisions_keep_original_identity_and_bind_markers_to_current_bytes() {
    let mut w = workspace(SOURCE);
    let original = w.root().unwrap().clone();
    let revised = SOURCE.replace("SUPER # modifier", "ALT # modifier");
    w.legacy_revision = Some(codec::Revision::new(SOURCE, revised.clone()).unwrap());
    assert!(w.validate(&catalogue()).is_err());
    w.legacy_binding_origin_sha256 = Some(hyprland_editor_core::lua::source_hash(&revised));
    let saved = w.to_json(&catalogue()).unwrap();
    let restored =
        Workspace::from_json(std::str::from_utf8(&saved).unwrap(), &catalogue()).unwrap();
    assert_eq!(restored.root().unwrap().content, original.content);
    assert_eq!(restored.root().unwrap().origin, original.origin);
    assert_eq!(restored.legacy_content(), Some(revised.as_str()));
    let mut bad = w.clone();
    bad.legacy_revision.as_mut().unwrap().base_sha256 = "0".repeat(64);
    assert!(bad.validate(&catalogue()).is_err());
    let mut bad = w.clone();
    bad.legacy_revision.as_mut().unwrap().schema_sha256 = "0".repeat(64);
    assert!(bad.validate(&catalogue()).is_err());
    let mut bad = w.clone();
    bad.sources[0].language = SourceLanguage::Lua;
    assert!(bad.validate(&catalogue()).is_err());
    let mut bad = w.clone();
    bad.sources[0].content.push(' ');
    assert!(bad.validate(&catalogue()).is_err());
}
#[test]
fn unsupported_definition_contexts_and_growth_bounds_do_not_hide_source() {
    let source="$bad-name = ignored\ngeneral {\n $nested = yes\n}\n$continued = one\\\n two\n# hyprlang if true\n$conditional = yes\n";
    let r = codec::inspect(source).unwrap();
    assert!(r.definitions.is_empty());
    assert_eq!(codec::edited_copy(source, &[]).unwrap(), source);
    let s = "$bad-name = ignored\n".repeat(5000);
    let r = codec::inspect(&s).unwrap();
    assert_eq!(r.unresolved_count, 5000);
    assert_eq!(r.unresolved.len(), 4096);
    let mut d = codec::Definition {
        id: "new".into(),
        name: "name".into(),
        value: "a".repeat(65537),
    };
    assert!(codec::validate(&[d.clone()]).is_err());
    d.value = "continued\\".into();
    assert!(codec::validate(&[d.clone()]).is_err());
    d.value = "one\ntwo".into();
    assert!(codec::validate(&[d]).is_err());
    let mut w = workspace(SOURCE);
    w.devices
        .push(hyprland_editor_core::device::Spec::new("lua", "separate"));
    w.device_schema_sha256 = Some(hyprland_editor_core::device::schema_hash());
    assert!(codec::prepare(
        &w,
        &codec::inspect(SOURCE).unwrap().definitions,
        &catalogue()
    )
    .is_err());
}
