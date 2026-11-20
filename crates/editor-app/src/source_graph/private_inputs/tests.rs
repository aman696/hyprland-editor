use super::*;
use hyprland_editor_core::{
    source::{RecordedOwnership, SourceIdentity},
    workspace_graph::{
        revisions::{Buffer, Piece, Revision},
        Graph,
    },
    AUDITED_MAIN, BASELINE,
};
fn source(id: &str, text: &str) -> SourceSnapshot {
    SourceSnapshot {
        id: id.into(),
        language: SourceLanguage::Lua,
        content: text.into(),
        origin: SourceIdentity {
            selected_path: format!("/inert-fixture/{id}.lua").into_bytes(),
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
fn fixture(version: &str, loader: &str) -> Bundle {
    let main = if loader == "loadfile" {
        "local f=loadfile('old');local part=f();hl.config{general={border_size=part}}".into()
    } else {
        format!("local part={loader}('old');hl.config{{general={{border_size=part}}}}")
    };
    let sources = vec![source("root", &main), source("part", "return 3")];
    let mut bundle = Bundle {
        graph: Graph {
            private: Default::default(),
            revisions: vec![],
            format: "hye-workspace-graph-1".into(),
            schema_sha256: workspace_graph::schema_sha256(),
            compiler: replay::compiler_profile(),
            root: "root".into(),
            context: Context {
                main_file: sources[0].origin.selected_path.clone(),
                home: None,
                inherited_package_path: None,
            },
            working_directory: None,
            views: sources
                .iter()
                .map(|s| View {
                    document: s.id.clone(),
                    load_name: s.origin.selected_path.clone(),
                })
                .collect(),
            bindings: vec![],
        },
        sources: Arc::new(sources),
    };
    let original = bundle
        .graph
        .program(&bundle.sources, Some("root"), version)
        .unwrap();
    let Some(Halt::NeedsResolution { request }) = replay::replay(&original).unwrap().halt else {
        panic!("original request")
    };
    bundle.graph.bindings.push(Binding {
        request,
        resolution: Resolution::Loaded {
            document: "part".into(),
            loader_data: if matches!(loader, "dofile" | "loadfile") {
                vec![]
            } else {
                bundle.sources[1].origin.selected_path.clone()
            },
        },
    });
    let at = main.find("old").unwrap();
    let private = main.replacen("old", "new", 1);
    bundle.graph.revisions.push(Revision {
        document: "root".into(),
        base_sha256: lua::source_hash(&main),
        sha256: lua::source_hash(&private),
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
                span: at + 3..main.len(),
            },
        ],
    });
    bundle
}
fn inspect(b: &Bundle, v: &str) -> Pending {
    inspect_pending(b, v, Arc::new(AtomicBool::new(false))).unwrap()
}
fn captured(p: &Pending, source: SourceSnapshot, load_name: Vec<u8>) -> Arc<CapturedInput> {
    Arc::new(CapturedInput {
        pending: p.fingerprint.clone(),
        request: p.request().unwrap().clone(),
        source: Arc::new(CapturedSource {
            content: source.content,
            identity: source.origin,
        }),
        loader_data: load_name.clone(),
        load_name,
        probes: vec![],
    })
}
#[test]
fn pending_reconstruction_drops_changed_rights_and_explicit_preserved_inputs_cover_all_loaders() {
    for version in [BASELINE, AUDITED_MAIN] {
        for loader in ["require", "__require", "dofile", "loadfile"] {
            let b = fixture(version, loader);
            let before = b.fingerprint().unwrap();
            assert!(restore(&b, version).is_err());
            let p = inspect(&b, version);
            assert_eq!(p.request().unwrap().module, b"new");
            assert!(p.program.bindings.is_empty());
            let next = accept_input(
                &p,
                InputChoice::Preserved("part".into()),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            assert!(next.request().is_none());
            assert!(next.report.halt.is_none());
            let rebuilt = restore(next.bundle(), version).unwrap();
            assert!(rebuilt.report.halt.is_none());
            assert_eq!(next.bundle.graph.private.bindings.len(), 1);
            assert_eq!(next.bundle.graph.bindings[0].request.module, b"old");
            assert_eq!(b.fingerprint().unwrap(), before);
            assert!(Arc::ptr_eq(&next.bundle.sources, &b.sources));
        }
    }
}
#[test]
fn fresh_folder_capture_for_each_loader_is_explicit_and_remains_data_after_grant_drop() {
    let _test = crate::capture_worker::TEST_LOCK.lock().unwrap();
    for version in [BASELINE, AUDITED_MAIN] {
        for loader in ["require", "__require", "dofile", "loadfile"] {
            let temp = crate::test_support::TempDir::new("he-private-input-capture");
            let mut bundle = fixture(version, loader);
            let root = temp
                .path()
                .join("root.lua")
                .as_os_str()
                .as_encoded_bytes()
                .to_vec();
            bundle.graph.context.main_file = root.clone();
            Arc::make_mut(&mut bundle.sources)[0].origin.selected_path = root.clone();
            bundle.graph.views[0].load_name = root;
            bundle.graph.working_directory =
                Some(temp.path().as_os_str().as_encoded_bytes().to_vec());
            let resolution = bundle.graph.bindings.remove(0).resolution;
            let original = bundle
                .graph
                .program(&bundle.sources, Some("root"), version)
                .unwrap();
            let Some(Halt::NeedsResolution { request }) = replay::replay(&original).unwrap().halt
            else {
                panic!("Original request")
            };
            bundle.graph.bindings.push(Binding {
                request,
                resolution,
            });
            let pending = inspect(&bundle, version);
            let target = temp
                .path()
                .join(if matches!(loader, "dofile" | "loadfile") {
                    "new"
                } else {
                    "new.lua"
                });
            std::fs::write(&target, "return 11\n").unwrap();
            assert!(capture_input(&pending, vec![], Arc::new(AtomicBool::new(false))).is_err());
            assert!(pending.bundle.graph.private.bindings.is_empty());
            let grant = Arc::new(directory_worker::approve(temp.path()).unwrap());
            let weak = Arc::downgrade(&grant);
            let captured =
                capture_input(&pending, vec![grant], Arc::new(AtomicBool::new(false))).unwrap();
            assert!(
                weak.upgrade().is_none(),
                "Capture result must not retain folder rights"
            );
            assert_eq!(captured.source().content, "return 11\n");
            assert!(captured.probe_count() >= 2);
            std::fs::write(&target, "return 12\n").unwrap();
            std::fs::remove_file(&target).unwrap();
            let next = accept_input(
                &pending,
                InputChoice::Capture(Arc::new(captured)),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            assert!(next.report.halt.is_none());
            assert_eq!(next.bundle.graph.private.sources[0].content, "return 11\n");
            assert!(restore(next.bundle(), version)
                .unwrap()
                .report
                .halt
                .is_none());
            assert_eq!(pending.request().unwrap().module, b"new");
        }
    }
}
#[test]
fn supplied_capture_data_never_substitutes_an_edited_or_equal_inserted_view() {
    for version in [BASELINE, AUDITED_MAIN] {
        for edited in ["return 7", "return 3"] {
            let mut b = fixture(version, "require");
            b.graph.revisions.push(Revision {
                document: "part".into(),
                base_sha256: lua::source_hash("return 3"),
                sha256: lua::source_hash(edited),
                buffers: vec![Arc::from(edited)],
                pieces: vec![Piece {
                    buffer: Buffer::Inserted { index: 0 },
                    span: 0..edited.len(),
                }],
            });
            let p = inspect(&b, version);
            let snapshot = b.sources[1].clone();
            let name = snapshot.origin.selected_path.clone();
            let data = captured(&p, snapshot, name);
            let next = accept_input(
                &p,
                InputChoice::Capture(data),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            assert_eq!(next.bundle.graph.private.sources.len(), 1);
            assert_eq!(next.bundle.graph.private.sources[0].content, "return 3");
            assert!(
                matches!(&next.bundle.graph.private.bindings[0].resolution,Resolution::Loaded {document,..} if document!="part")
            );
            assert!(next
                .program
                .documents
                .iter()
                .any(|d| d.id == "part" && d.source == edited));
            assert!(restore(next.bundle(), version)
                .unwrap()
                .report
                .halt
                .is_none());
        }
    }
}
#[test]
fn archived_data_activates_only_after_explicit_preserved_selection() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut b = fixture(version, "require");
        let s = source("archived", "return 8");
        b.graph.private.views.push(View {
            document: s.id.clone(),
            load_name: s.origin.selected_path.clone(),
        });
        b.graph.private.archived.insert(s.id.clone());
        Arc::make_mut(&mut b.graph.private.sources).push(s);
        let p = inspect(&b, version);
        assert!(!p.program.documents.iter().any(|d| d.id == "archived"));
        assert!(p
            .preserved_program()
            .unwrap()
            .documents
            .iter()
            .any(|d| d.id == "archived"));
        let next = accept_input(
            &p,
            InputChoice::Preserved("archived".into()),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert!(p.bundle.graph.private.archived.contains("archived"));
        assert!(!next.bundle.graph.private.archived.contains("archived"));
        assert!(restore(next.bundle(), version)
            .unwrap()
            .report
            .halt
            .is_none());
    }
}
#[test]
fn missing_is_explicit_and_does_not_turn_unknown_folders_into_absence() {
    for version in [BASELINE, AUDITED_MAIN] {
        let b = fixture(version, "require");
        let p = inspect(&b, version);
        assert!(capture_input(&p, vec![], Arc::new(AtomicBool::new(false)))
            .err()
            .unwrap()
            .contains("approved folder"));
        assert!(p.bundle.graph.private.bindings.is_empty());
        let next =
            accept_input(&p, InputChoice::Missing, Arc::new(AtomicBool::new(false))).unwrap();
        assert!(next
            .report
            .loads
            .iter()
            .any(|l| matches!(l.outcome, replay::LoadOutcome::SuppliedMissing)));
        assert!(matches!(
            next.bundle.graph.private.bindings[0].resolution,
            Resolution::Missing
        ));
        assert!(restore(next.bundle(), version).is_ok());
    }
}
#[test]
fn foreign_captured_requests_cancelled_queries_and_unknown_views_leave_inputs_unchanged() {
    for version in [BASELINE, AUDITED_MAIN] {
        let b = fixture(version, "require");
        let before = b.fingerprint().unwrap();
        let p = inspect(&b, version);
        let snapshot = source("new", "return 9");
        let name = snapshot.origin.selected_path.clone();
        let mut data = captured(&p, snapshot, name);
        Arc::get_mut(&mut data).unwrap().pending = "foreign".into();
        assert!(accept_input(
            &p,
            InputChoice::Capture(data),
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
        assert!(accept_input(
            &p,
            InputChoice::Preserved("unknown".into()),
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
        assert!(inspect_pending(&b, version, Arc::new(AtomicBool::new(true))).is_err());
        assert_eq!(b.fingerprint().unwrap(), before);
    }
}
#[test]
fn additional_capture_at_document_limit_refuses_without_discarding_archives() {
    for version in [BASELINE, AUDITED_MAIN] {
        let mut b = fixture(version, "require");
        for n in 0..30 {
            let s = source(&format!("archive-{n}"), "return 0");
            b.graph.private.views.push(View {
                document: s.id.clone(),
                load_name: s.origin.selected_path.clone(),
            });
            b.graph.private.archived.insert(s.id.clone());
            Arc::make_mut(&mut b.graph.private.sources).push(s);
        }
        let p = inspect(&b, version);
        let before = p.bundle.fingerprint().unwrap();
        let snapshot = source("new", "return 9");
        let name = snapshot.origin.selected_path.clone();
        assert!(accept_input(
            &p,
            InputChoice::Capture(captured(&p, snapshot, name)),
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
        assert_eq!(p.bundle.fingerprint().unwrap(), before);
        assert_eq!(p.bundle.graph.private.archived.len(), 30);
    }
}
