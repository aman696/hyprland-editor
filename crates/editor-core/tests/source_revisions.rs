use hyprland_editor_core::{
    lua::{
        replay::{edits::*, *},
        source_hash,
    },
    workspace_graph::revisions::{self, Buffer, Piece, Revision as SourceRevision},
    BASELINE, MAX_DOCUMENT_BYTES,
};
use std::sync::Arc;
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
        initial_package_path: None,
        initial_package_path_unknown_tail: false,
        working_directory: None,
    }
}
fn edit(p: &Program, id: &str, old: &str, new: &str) -> SourceEdit {
    let d = p.documents.iter().find(|d| d.id == id).unwrap();
    let start = d.source.find(old).unwrap();
    SourceEdit {
        document: id.into(),
        base_sha256: d.sha256.clone(),
        span: start..start + old.len(),
        old_bytes_sha256: source_hash(old),
        replacement: new.into(),
        semantic_targets: vec!["fixture.value".into()],
    }
}
#[test]
fn sequential_piece_changes_share_inserted_buffers_and_can_restore_captured_bytes() {
    let base = program("-- café\r\nlocal x=2;emit(x)\r\n");
    let original = base.documents[0].source.clone();
    let first = revisions::apply(
        &base,
        &[],
        &prepare(&base, vec![edit(&base, "root", "2", "222")], vec![]).unwrap(),
    )
    .unwrap();
    let current = revisions::program(base.clone(), &first).unwrap();
    let mut e = edit(&current, "root", "222", "9");
    e.span.start += 1;
    e.span.end -= 1;
    e.old_bytes_sha256 = source_hash("2");
    let second =
        revisions::apply(&base, &first, &prepare(&current, vec![e], vec![]).unwrap()).unwrap();
    assert!(Arc::ptr_eq(&first[0].buffers[0], &second[0].buffers[0]));
    let current = revisions::program(base.clone(), &second).unwrap();
    assert_eq!(
        current.documents[0].source,
        original.replacen('2', "292", 1)
    );
    assert_eq!(base.documents[0].source, original);
    let encoded = serde_json::to_string(&second).unwrap();
    assert_eq!(encoded.matches("\"222\"").count(), 1);
    let decoded: Vec<SourceRevision> = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        revisions::program(base.clone(), &decoded)
            .unwrap()
            .documents[0]
            .source,
        current.documents[0].source
    );
    let restored = revisions::apply(
        &base,
        &second,
        &prepare(&current, vec![edit(&current, "root", "292", "2")], vec![]).unwrap(),
    )
    .unwrap();
    assert!(restored.is_empty());
}
#[test]
fn unchanged_loader_pieces_rebase_offsets_and_hashes_without_new_resolution_rights() {
    let mut base = program("local n=require('part');emit(n)");
    let module = "return 4";
    base.documents.push(Document {
        id: "part".into(),
        source: module.into(),
        sha256: source_hash(module),
        load_name: b"/fixture/part.lua".to_vec(),
    });
    let Some(Halt::NeedsResolution { request }) = replay(&base).unwrap().halt else {
        panic!("Missing request")
    };
    base.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: b"/fixture/part.lua".to_vec(),
        },
    });
    let prefix = "-- prefix\n";
    let plan = prepare(
        &base,
        vec![
            edit(&base, "root", "", prefix),
            edit(&base, "part", "4", "6"),
        ],
        vec![],
    )
    .unwrap();
    let revisions = revisions::apply(&base, &[], &plan).unwrap();
    let p = revisions::program(base.clone(), &revisions).unwrap();
    assert_eq!(
        p.bindings[0].request.span.start,
        base.bindings[0].request.span.start + prefix.len()
    );
    assert_eq!(p.bindings[0].request.sha256, p.documents[0].sha256);
    assert_eq!(
        replay(&p).unwrap().events[0].arguments,
        vec![Observed::Int(6)]
    );
    let plan = prepare(
        &base,
        vec![edit(&base, "root", "'part'", "'other'")],
        vec![],
    )
    .unwrap();
    assert!(revisions::apply(&base, &[], &plan)
        .unwrap_err()
        .contains("loader request"));
}
#[test]
fn deleted_calls_and_incomplete_models_remain_private_data_without_dropping_base_sources() {
    let base = program("if true then emit(4)end");
    let plan = prepare(&base, vec![edit(&base, "root", "true", "false")], vec![]).unwrap();
    let r = revisions::apply(&base, &[], &plan).unwrap();
    let p = revisions::program(base.clone(), &r).unwrap();
    assert!(replay(&p).unwrap().events.is_empty());
    assert_eq!(base.documents[0].source, "if true then emit(4)end");
    let p = program("emit(4)");
    let plan = prepare(
        &p,
        vec![edit(&p, "root", "4", "4);os.execute('not executed')--")],
        vec![],
    )
    .unwrap();
    let r = revisions::apply(&p, &[], &plan).unwrap();
    assert!(matches!(
        replay(&revisions::program(p, &r).unwrap()).unwrap().halt,
        Some(Halt::Unresolved { .. })
    ));
}
#[test]
fn forged_piece_hashes_buffers_ranges_duplicate_ids_and_stale_plans_are_refused() {
    let base = program("-- é\nemit(2)");
    let plan = prepare(&base, vec![edit(&base, "root", "2", "44")], vec![]).unwrap();
    let r = revisions::apply(&base, &[], &plan).unwrap();
    for case in 0..6 {
        let mut bad = r.clone();
        match case {
            0 => bad[0].base_sha256 = source_hash("foreign"),
            1 => bad[0].sha256 = source_hash("foreign"),
            2 => bad[0].buffers.push(Arc::from("unused")),
            3 => bad[0].pieces[0].span.start = 4,
            4 => bad[0].pieces.push(Piece {
                buffer: Buffer::Original,
                span: 0..1,
            }),
            _ => bad[0].pieces[0].buffer = Buffer::Inserted { index: 99 },
        }
        assert!(
            revisions::program(base.clone(), &bad).is_err(),
            "case {case}"
        );
    }
    assert!(revisions::program(base.clone(), &[r[0].clone(), r[0].clone()]).is_err());
    assert!(revisions::apply(&base, &r, &plan).is_err());
}
#[test]
fn inserted_source_limits_and_unreached_syntax_errors_are_checked() {
    let base = program("emit(2)");
    let mut p = prepare(&base, vec![edit(&base, "root", "2", "3")], vec![]).unwrap();
    p.edits[0].replacement = " ".repeat(MAX_DOCUMENT_BYTES + 1);
    assert!(revisions::apply(&base, &[], &p).is_err());
    let mut base = base;
    let source = "return 2";
    base.documents.push(Document {
        id: "unused".into(),
        source: source.into(),
        sha256: source_hash(source),
        load_name: b"/fixture/unused.lua".to_vec(),
    });
    let p = prepare(&base, vec![edit(&base, "unused", "2", "(")], vec![]).unwrap();
    assert!(revisions::apply(&base, &[], &p).is_err());
}
