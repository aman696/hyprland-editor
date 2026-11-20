use hyprland_editor_core::{
    lua::{
        self,
        replay::{self, *},
    },
    source::{RecordedOwnership, SourceIdentity},
    workspace::{SourceLanguage, SourceSnapshot},
    workspace_graph::{
        private::Inputs,
        revisions::{self, Buffer, Piece, Revision},
        View,
    },
    AUDITED_MAIN, BASELINE,
};
use std::sync::Arc;
fn snapshot(id: &str, text: &str) -> SourceSnapshot {
    SourceSnapshot {
        id: id.into(),
        language: SourceLanguage::Lua,
        content: text.into(),
        origin: SourceIdentity {
            selected_path: format!("/private/{id}.lua").into_bytes(),
            resolved_path: None,
            links: vec![],
            device: 1,
            inode: 2,
            owner: 1000,
            mode: 0o100600,
            size: text.len() as u64,
            modified_seconds: 0,
            modified_nanoseconds: 0,
            changed_seconds: 0,
            changed_nanoseconds: 0,
            sha256: lua::source_hash(text),
            recorded_ownership: RecordedOwnership::UserOwned,
        },
    }
}
fn base(version: &str, loader: &str) -> Program {
    let source = if loader == "loadfile" {
        "local fn=loadfile('part');local p=fn();hl.config{general={border_size=p}}".into()
    } else {
        format!("local p={loader}('part');hl.config{{general={{border_size=p}}}}")
    };
    let mut p = Program {
        version: version.into(),
        profile: Profile::HyprlandLua54,
        root: "root".into(),
        documents: vec![Document {
            id: "root".into(),
            sha256: lua::source_hash(&source),
            source,
            load_name: b"/captured/main.lua".to_vec(),
        }],
        bindings: vec![],
        initial_package_path: Some(b"/captured/?.lua".to_vec()),
        initial_package_path_unknown_tail: false,
        working_directory: Some(b"/captured".to_vec()),
    };
    let Some(Halt::NeedsResolution { request }) = replay::replay(&p).unwrap().halt else {
        panic!("initial loader")
    };
    let direct = matches!(request.loader, Loader::Dofile | Loader::Loadfile);
    let name = if direct {
        b"part".to_vec()
    } else {
        b"/captured/part.lua".to_vec()
    };
    p.documents.push(Document {
        id: "part".into(),
        source: "return 3".into(),
        sha256: lua::source_hash("return 3"),
        load_name: name.clone(),
    });
    p.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: if direct { vec![] } else { name },
        },
    });
    assert!(replay::replay(&p).unwrap().halt.is_none());
    p
}
fn changed(p: &Program) -> Revision {
    let doc = &p.documents[0];
    let at = doc.source.find("part").unwrap();
    let content = format!("{}new{}", &doc.source[..at], &doc.source[at + 4..]);
    Revision {
        document: "root".into(),
        base_sha256: doc.sha256.clone(),
        sha256: lua::source_hash(&content),
        buffers: vec![Arc::from("new")],
        pieces: vec![
            Piece {
                buffer: Buffer::Original,
                span: 0..at,
            },
            Piece {
                buffer: Buffer::Inserted { index: 0 },
                span: 0..3,
            },
            Piece {
                buffer: Buffer::Original,
                span: at + 4..doc.source.len(),
            },
        ],
    }
}
fn resolved(p: &Program) -> (Vec<Revision>, Inputs) {
    let revisions = vec![changed(p)];
    let source = snapshot("private-part", "return 8");
    let direct = matches!(
        p.bindings[0].request.loader,
        Loader::Dofile | Loader::Loadfile
    );
    let name = if direct {
        b"new".to_vec()
    } else {
        source.origin.selected_path.clone()
    };
    let mut inputs = Inputs {
        views: vec![View {
            document: source.id.clone(),
            load_name: name.clone(),
        }],
        sources: Arc::new(vec![source]),
        ..Inputs::default()
    };
    let pending = inputs.program(p, &revisions).unwrap();
    assert!(pending.bindings.is_empty());
    let Some(Halt::NeedsResolution { request }) = replay::replay(&pending).unwrap().halt else {
        panic!("renewal request")
    };
    inputs.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "private-part".into(),
            loader_data: if direct { vec![] } else { name },
        },
    });
    (revisions, inputs)
}
fn check(inputs: &Inputs, p: &Program, pieces: &[Revision]) -> Program {
    let current = inputs.program(p, pieces).unwrap();
    let report = replay::replay(&current).unwrap();
    inputs.check_reached(&current, &report).unwrap();
    assert!(report.halt.is_none());
    current
}
fn plan(
    p: &Program,
    doc: &str,
    span: std::ops::Range<usize>,
    replacement: &str,
) -> edits::EditPlan {
    let d = p.documents.iter().find(|d| d.id == doc).unwrap();
    edits::prepare(
        p,
        vec![edits::SourceEdit {
            document: doc.into(),
            base_sha256: d.sha256.clone(),
            old_bytes_sha256: lua::source_hash(&d.source[span.clone()]),
            span,
            replacement: replacement.into(),
            semantic_targets: vec!["reviewed".into()],
        }],
        vec![],
    )
    .unwrap()
}
#[test]
fn changed_imports_require_explicit_private_inputs_for_all_four_loader_modes() {
    for version in [BASELINE, AUDITED_MAIN] {
        for loader in ["require", "__require", "dofile", "loadfile"] {
            let p = base(version, loader);
            let fingerprint = serde_json::to_vec(&p).unwrap();
            let (pieces, inputs) = resolved(&p);
            assert!(Inputs::default().program(&p, &pieces).is_err());
            let pending = Inputs::default().pending_program(&p, &pieces).unwrap();
            assert!(pending.bindings.is_empty());
            assert!(
                matches!(replay::replay(&pending).unwrap().halt,Some(Halt::NeedsResolution {request}) if request.module==b"new")
            );
            let current = check(&inputs, &p, &pieces);
            assert_eq!(serde_json::to_vec(&p).unwrap(), fingerprint);
            assert_eq!(current.bindings.len(), 1);
            assert_eq!(current.bindings[0].request.module, b"new");
            assert_eq!(current.documents[1].source, "return 3");
            assert_eq!(current.documents[2].source, "return 8");
            let report = replay::replay(&current).unwrap();
            assert_eq!(
                report.events[0].arguments[0],
                Observed::Table(vec![(
                    Observed::Bytes(b"general".to_vec()),
                    Observed::Table(vec![(
                        Observed::Bytes(b"border_size".to_vec()),
                        Observed::Int(8)
                    )])
                )])
            );
        }
    }
}
#[test]
fn private_source_edits_and_root_offsets_preserve_original_capture_bytes_and_explicit_bindings() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = base(version, "require");
        let (mut pieces, mut inputs) = resolved(&p);
        let sources = inputs.sources.clone();
        let current = check(&inputs, &p, &pieces);
        let edit = plan(&current, "private-part", 7..8, "9");
        (pieces, inputs) = inputs.apply(&p, &pieces, &edit).unwrap();
        let current = check(&inputs, &p, &pieces);
        assert_eq!(current.documents[2].source, "return 9");
        let before = current.bindings[0].request.span.clone();
        let prefix = "-- private prefix\n";
        let edit = plan(&current, "root", 0..0, prefix);
        (pieces, inputs) = inputs.apply(&p, &pieces, &edit).unwrap();
        let current = check(&inputs, &p, &pieces);
        assert_eq!(
            current.bindings[0].request.span.start,
            before.start + prefix.len()
        );
        assert!(Arc::ptr_eq(&sources, &inputs.sources));
        assert_eq!(inputs.sources[0].content, "return 8");
        assert_eq!(
            inputs.sources[0].origin.sha256,
            lua::source_hash("return 8")
        );
    }
}
#[test]
fn removing_private_import_archives_captured_data_without_retaining_active_bindings() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = base(version, "require");
        let (pieces, inputs) = resolved(&p);
        let current = check(&inputs, &p, &pieces);
        let request = current.bindings[0].request.clone();
        let edit = plan(&current, "root", request.span, "8");
        let (pieces, next) = inputs.apply(&p, &pieces, &edit).unwrap();
        let current = check(&next, &p, &pieces);
        assert!(next.bindings.is_empty());
        assert!(next.archived.contains("private-part"));
        assert!(Arc::ptr_eq(&inputs.sources, &next.sources));
        assert!(current.documents.iter().all(|d| d.id != "private-part"));
        assert!(current.bindings.is_empty());
    }
}
#[test]
fn stale_contexts_unreached_targets_and_archived_or_duplicate_inputs_are_refused() {
    let p = base(BASELINE, "require");
    let (pieces, inputs) = resolved(&p);
    let mut bad = inputs.clone();
    bad.bindings[0].request.sha256 = "0".repeat(64);
    assert!(bad.program(&p, &pieces).is_err());
    bad = inputs.clone();
    bad.bindings.push(bad.bindings[0].clone());
    assert!(bad.program(&p, &pieces).is_err());
    bad = inputs.clone();
    bad.bindings[0].request.package_path = Some(b"/other/?.lua".to_vec());
    let current = bad.program(&p, &pieces).unwrap();
    assert!(bad
        .check_reached(&current, &replay::replay(&current).unwrap())
        .is_err());
    bad = inputs.clone();
    bad.archived.insert("private-part".into());
    assert!(bad.program(&p, &pieces).is_err());
    bad = inputs.clone();
    bad.archived.insert("unknown".into());
    assert!(bad.program(&p, &pieces).is_err());
    bad = inputs.clone();
    bad.views[0].document = "unknown".into();
    assert!(bad.program(&p, &pieces).is_err());
    bad = inputs.clone();
    bad.views[0].load_name.push(0);
    assert!(bad.program(&p, &pieces).is_err());
    bad = inputs.clone();
    bad.bindings.clear();
    let current = bad.program(&p, &pieces).unwrap();
    assert!(bad
        .check_reached(&current, &replay::replay(&current).unwrap())
        .is_err());
    let current = check(&inputs, &p, &pieces);
    let mut report = replay::replay(&current).unwrap();
    report.runtime_verified = true;
    assert!(inputs.check_reached(&current, &report).is_err());
    report.runtime_verified = false;
    report.profile = Profile::SyntheticEmit;
    assert!(inputs.check_reached(&current, &report).is_err());
}
#[test]
fn explicit_missing_and_archived_data_do_not_create_filesystem_or_execution_authority() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = base(version, "require");
        let (pieces, mut inputs) = resolved(&p);
        inputs.bindings[0].resolution = Resolution::Missing;
        inputs.archived.insert("private-part".into());
        let current = inputs.program(&p, &pieces).unwrap();
        let report = replay::replay(&current).unwrap();
        inputs.check_reached(&current, &report).unwrap();
        assert!(report
            .loads
            .iter()
            .any(|l| matches!(l.outcome, LoadOutcome::SuppliedMissing)));
        assert!(!report.runtime_verified);
        let mut archived = Inputs::default();
        let source = snapshot("historical", "not valid Lua !");
        archived.views.push(View {
            document: source.id.clone(),
            load_name: b"/historical.lua".to_vec(),
        });
        archived.archived.insert(source.id.clone());
        archived.sources = Arc::new(vec![source]);
        let current = check(&archived, &p, &[]);
        assert_eq!(current.documents.len(), p.documents.len());
    }
}
#[test]
fn private_safe_require_compile_errors_remain_modeled_observations() {
    for version in [BASELINE, AUDITED_MAIN] {
        let p = base(version, "require");
        let (pieces, mut inputs) = resolved(&p);
        let replacement = snapshot("private-part", "local = broken Lua");
        inputs.sources = Arc::new(vec![replacement]);
        let current = inputs.program(&p, &pieces).unwrap();
        let report = replay::replay(&current).unwrap();
        inputs.check_reached(&current, &report).unwrap();
        assert!(report.logged_module_errors > 0);
        assert!(!report.runtime_verified);
    }
}
#[test]
fn archived_private_data_cannot_bypass_document_or_aggregate_byte_limits() {
    let p = base(BASELINE, "require");
    let make = |count: usize, text: &str| {
        let sources = (0..count)
            .map(|n| snapshot(&format!("archived-{n}"), text))
            .collect::<Vec<_>>();
        Inputs {
            views: sources
                .iter()
                .map(|s| View {
                    document: s.id.clone(),
                    load_name: s.origin.selected_path.clone(),
                })
                .collect(),
            archived: sources.iter().map(|s| s.id.clone()).collect(),
            sources: Arc::new(sources),
            bindings: vec![],
        }
    };
    assert!(make(30, "return 1").program(&p, &[]).is_ok());
    assert!(make(31, "return 1").program(&p, &[]).is_err());
    let large = " ".repeat(1024 * 1024);
    assert!(make(3, &large).program(&p, &[]).is_ok());
    assert!(make(4, &large).program(&p, &[]).is_err());
}
#[test]
fn byte_equal_private_replacements_never_restore_captured_provenance() {
    let mut p = base(BASELINE, "require");
    p.bindings.clear();
    p.documents = vec![Document {
        id: "root".into(),
        source: "return 1".into(),
        sha256: lua::source_hash("return 1"),
        load_name: b"/root.lua".to_vec(),
    }];
    let first = plan(&p, "root", 7..8, "2");
    let pieces = revisions::prepare_pieces(&p, &[], &first).unwrap();
    let current = revisions::program(p.clone(), &pieces).unwrap();
    let second = plan(&current, "root", 7..8, "1");
    let pieces = revisions::prepare_pieces(&p, &pieces, &second).unwrap();
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].sha256, p.documents[0].sha256);
    assert!(!pieces[0].buffers.is_empty());
}
