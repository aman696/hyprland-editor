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
            load_name: b"/fixture/main.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: Some(b"/fixture/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn snippet<'a>(p: &'a Program, r: &Report, id: u32) -> &'a str {
    let e = &r.expression_sources[id as usize];
    &p.documents[e.document].source[e.span.clone()]
}
fn row(r: &Report, event: usize) -> (&ValueSource, &KeySource) {
    let s = &r.events[event].argument_sources[0];
    (&s.rows[0], &s.keys[0])
}
#[test]
fn table_keys_keep_first_insertion_latest_address_and_delete_reinsert_history_separate() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut p=program("local first='border_size';local second=first;local t={[first]=1};local alias=t;alias[second]=2;emit(t);rawset(alias,second,nil);emit(t);alias[second]=3;emit(t)");
        p.version = version.into();
        let r = replay(&p).unwrap();
        assert!(r.halt.is_none(), "{:?}", r.halt);
        let (value, key) = row(&r, 0);
        let first = key.introduced_by.unwrap();
        let latest = key.latest_write.unwrap();
        assert_ne!(first, latest);
        assert_eq!(value.storage_write, Some(latest));
        let a = &r.storage_writes[first as usize];
        let b = &r.storage_writes[latest as usize];
        assert_eq!(a.kind, WriteKind::TableConstructor);
        assert_eq!(b.kind, WriteKind::Assignment);
        assert_eq!(b.previous, Some(first));
        assert_eq!(a.place, b.place);
        assert_eq!(snippet(&p, &r, a.key_expression.unwrap()), "first");
        assert_eq!(snippet(&p, &r, b.key_expression.unwrap()), "second");
        assert_eq!(snippet(&p, &r, b.receiver_expression.unwrap()), "alias");
        assert_eq!(snippet(&p, &r, b.value_expression.unwrap()), "2");
        assert!(r.events[1].argument_sources[0].rows.is_empty());
        let deletion = r
            .storage_writes
            .iter()
            .position(|w| w.kind == WriteKind::Rawset)
            .unwrap() as u32;
        assert!(r.storage_writes[deletion as usize].deletes_slot);
        assert_eq!(r.storage_writes[deletion as usize].previous, Some(latest));
        let (_, key) = row(&r, 2);
        let reinsert = key.latest_write.unwrap();
        assert_eq!(key.introduced_by, Some(reinsert));
        assert_ne!(reinsert, first);
        assert_eq!(r.storage_writes[reinsert as usize].previous, Some(deletion));
        assert_eq!(r.storage_writes[latest as usize].at_order, 0);
        assert_eq!(r.storage_writes[deletion as usize].at_order, 1);
        assert_eq!(r.storage_writes[reinsert as usize].at_order, 2);
        let roundtrip: Report = serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();
        assert_eq!(roundtrip.storage_writes, r.storage_writes);
    }
}
#[test]
fn lexical_cells_parameter_bindings_and_nil_assignments_do_not_merge_by_name_or_value() {
    let p=program("local x=1;local function f(x)x=x+1;return x end;do local x=4;emit(x)end;emit(f(x));x=nil;emit(x)");
    let r = replay(&p).unwrap();
    assert!(r.halt.is_none());
    let inner = r.events[0].argument_sources[0].storage_write.unwrap();
    let changed = r.events[1].argument_sources[0].storage_write.unwrap();
    let parameter = r.storage_writes[changed as usize].previous.unwrap();
    assert_eq!(
        r.storage_writes[parameter as usize].kind,
        WriteKind::Parameter
    );
    let outer = r.storage_writes[parameter as usize].value_write.unwrap();
    assert_ne!(
        r.storage_writes[inner as usize].place,
        r.storage_writes[outer as usize].place
    );
    assert_ne!(
        r.storage_writes[parameter as usize].place,
        r.storage_writes[outer as usize].place
    );
    assert_eq!(
        snippet(
            &p,
            &r,
            r.storage_writes[outer as usize].value_expression.unwrap()
        ),
        "1"
    );
    let nil = r.events[2].argument_sources[0].storage_write.unwrap();
    assert!(!r.storage_writes[nil as usize].deletes_slot);
    assert_eq!(r.storage_writes[nil as usize].previous, Some(outer));
    let p = program("local t={};local k='x';t[k],t['x']=1,2;emit('after')");
    let r = replay(&p).unwrap();
    assert!(
        matches!(r.halt,Some(Halt::Unresolved{ref reason,..}) if reason.contains("Repeated assignment"))
    );
    assert!(r.events.is_empty());
}
#[test]
fn cached_module_aliases_keep_shared_table_identity_and_cross_document_field_writes() {
    let mut p = program("local t=require('part');local a=require('part');t.n=5;emit(a)");
    let module = "local shared={n=1};return shared";
    p.documents.push(Document {
        id: "part".into(),
        source: module.into(),
        sha256: source_hash(module),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let Some(Halt::NeedsResolution { request }) = replay(&p).unwrap().halt else {
        panic!("No module request")
    };
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let r = replay(&p).unwrap();
    assert!(r.halt.is_none());
    assert!(matches!(r.loads[1].outcome, LoadOutcome::Cached));
    let (value, key) = row(&r, 0);
    let old = &r.storage_writes[key.introduced_by.unwrap() as usize];
    let new = &r.storage_writes[key.latest_write.unwrap() as usize];
    assert_eq!(old.site.as_ref().unwrap().document, 1);
    assert_eq!(new.site.as_ref().unwrap().document, 0);
    assert_eq!(old.place, new.place);
    assert_eq!(value.storage_write, key.latest_write);
    assert_eq!(
        r.expression_sources[old.key_expression.unwrap() as usize].kind,
        ExpressionKind::LiteralKey
    );
    assert!(r
        .storage_writes
        .iter()
        .any(|w| w.kind == WriteKind::ModeledIntrinsic && w.value_table.is_some()));
}
#[test]
fn provenance_growth_fails_without_later_effects_or_dangling_write_references() {
    let p =
        program("local t={};pcall(function()for i=1,10000 do rawset(t,i,i)end end);emit('after')");
    let r = replay(&p).unwrap();
    assert!(matches!(r.halt, Some(Halt::Budget { .. })));
    assert!(r.events.is_empty());
    assert!(r.storage_writes.len() <= 8192);
    assert!(serde_json::to_vec(&r).unwrap().len() < 2 * 1024 * 1024);
    for (index, write) in r.storage_writes.iter().enumerate() {
        for id in [write.previous, write.value_write, write.receiver_write]
            .into_iter()
            .flatten()
        {
            assert!((id as usize) < index);
        }
        for id in [
            write.value_expression,
            write.key_expression,
            write.receiver_expression,
        ]
        .into_iter()
        .flatten()
        {
            assert!((id as usize) < r.expression_sources.len());
        }
        if let Some(site) = &write.site {
            assert!(p.documents[site.document]
                .source
                .get(site.span.clone())
                .is_some());
        }
    }
}
