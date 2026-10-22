use super::*;
fn lcs(a: &[u32], b: &[u32], ban: Option<(usize, usize)>) -> usize {
    let mut row = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        let mut diagonal = 0;
        for (j, y) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if x == y && ban != Some((i, j)) {
                diagonal + 1
            } else {
                above.max(row[j])
            };
            diagonal = above;
        }
    }
    row[b.len()]
}
fn words(max: usize) -> Vec<Vec<u32>> {
    let mut all = vec![];
    for n in 0..=max {
        for pattern in 0..1 << n {
            all.push((0..n).map(|i| ((pattern >> i) & 1) as u32).collect());
        }
    }
    all
}
#[test]
fn shortest_scripts_and_mandatory_anchors_match_an_independent_exhaustive_lcs_oracle() {
    let words = words(5);
    for a in &words {
        for b in &words {
            let script = diff::script(a, b, &mut Budget::new()).unwrap();
            assert_eq!(
                script.distance,
                a.len() + b.len() - 2 * lcs(a, b, None),
                "{a:?} -> {b:?}"
            );
            let mut output = vec![];
            let mut at = 0;
            for h in &script.hunks {
                output.extend_from_slice(&a[at..h.old.start]);
                output.extend_from_slice(&b[h.new.clone()]);
                at = h.old.end;
            }
            output.extend_from_slice(&a[at..]);
            assert_eq!(&output, b);
            for (old, new) in &script.equals {
                for old in old.clone() {
                    let new = new.start + old
                        - script
                            .equals
                            .iter()
                            .find(|(r, _)| r.contains(&old))
                            .unwrap()
                            .0
                            .start;
                    let alternative =
                        diff::alternative(a, b, (old, new), script.distance, &mut Budget::new())
                            .unwrap();
                    assert_eq!(
                        alternative,
                        lcs(a, b, Some((old, new))) == lcs(a, b, None),
                        "{a:?} -> {b:?}, ban {old},{new}"
                    );
                }
            }
        }
    }
}
fn automatic(base: &str, private: &str, incoming: &str) -> String {
    let review = prepare(base, private, incoming).unwrap();
    assert!(
        review.regions.iter().all(|r| r.automatic.is_some()),
        "{review:?}"
    );
    resolve(base, private, incoming, &review, &BTreeMap::new())
        .unwrap()
        .content
}
#[test]
fn disjoint_lines_and_components_merge_without_normalizing_any_bytes() {
    let base="-- header é\r\nhl.monitor{output='DP-1',transform=1,scale='1'}\r\nhl.config{general={border_size=2}}\r\n-- footer";
    let private = base.replace("transform=1", "transform=3");
    let incoming = base
        .replace("scale='1'", "scale='auto'")
        .replace("border_size=2", "border_size=4");
    assert_eq!(
        automatic(base, &private, &incoming),
        private
            .replace("scale='1'", "scale='auto'")
            .replace("border_size=2", "border_size=4")
    );
    assert_eq!(
        automatic("a\nb\nc\n", "A\nb\nc\n", "a\nb\nC\n"),
        "A\nb\nC\n"
    );
    assert_eq!(
        automatic("print('é')\n", "print('É')\n", "print('é')\n-- new"),
        "print('É')\n-- new"
    );
}
#[test]
fn overlap_and_concurrent_insertions_require_every_explicit_choice() {
    for (base, left, right) in [
        ("value=1\n", "value=2\n", "value=3\n"),
        ("", "left\n", "right\n"),
    ] {
        let review = prepare(base, left, right).unwrap();
        assert!(review.regions.iter().any(|r| r.automatic.is_none()));
        assert!(resolve(base, left, right, &review, &BTreeMap::new()).is_err());
        for (choice, wanted) in [
            (Choice::Private, left),
            (Choice::Incoming, right),
            (Choice::Base, base),
        ] {
            let choices = review
                .regions
                .iter()
                .enumerate()
                .filter(|(_, r)| r.automatic.is_none())
                .map(|(i, _)| (i, choice))
                .collect();
            assert_eq!(
                resolve(base, left, right, &review, &choices)
                    .unwrap()
                    .content,
                wanted
            );
        }
    }
    let review = prepare("", "a\n", "b\n").unwrap();
    let choice = [(0, Choice::PrivateThenIncoming)].into();
    assert_eq!(
        resolve("", "a\n", "b\n", &review, &choice).unwrap().content,
        "a\nb\n"
    );
}
#[test]
fn repeated_line_placement_is_not_guessed_when_an_alternative_anchor_exists() {
    let base = "same\nseparator\nsame\ntrailer\n";
    let left = "same\n";
    let right = "changed\nseparator\nsame\ntrailer\n";
    let review = prepare(base, left, right).unwrap();
    assert!(
        review
            .regions
            .iter()
            .any(|r| r.conflict == Some(Conflict::AmbiguousAlignment)),
        "{review:?}"
    );
    let base = "same\nseparator\nsame\ntrailer\n";
    assert_eq!(
        automatic(
            base,
            "same\nseparator\ntrailer\n",
            "changed\nseparator\nsame\ntrailer\n"
        ),
        "changed\nseparator\ntrailer\n"
    );
}
#[test]
fn stale_inputs_forged_regions_foreign_choices_and_budgets_fail_without_output() {
    let review = prepare("a\n", "b\n", "c\n").unwrap();
    assert!(resolve("changed", "b\n", "c\n", &review, &BTreeMap::new()).is_err());
    let mut bad = review.clone();
    bad.regions.clear();
    assert!(resolve("a\n", "b\n", "c\n", &bad, &BTreeMap::new()).is_err());
    assert!(resolve(
        "a\n",
        "b\n",
        "c\n",
        &review,
        &[(999, Choice::Private)].into()
    )
    .is_err());
    assert!(prepare("\0", "", "").is_err());
    assert!(prepare(&"a".repeat(MAX_DOCUMENT_BYTES + 1), "", "").is_err());
    let mut budget = Budget {
        remaining: 1,
        peak: 0,
        anchors: 0,
    };
    assert!(diff::script(&[1, 2, 3], &[4, 5, 6], &mut budget).is_err());
}
#[test]
fn no_op_empty_final_newline_and_equal_changes_are_exact_and_keep_data_provenance() {
    for text in ["", "-- é", "\r\n", "a\n\n\r\nb"] {
        let review = prepare(text, text, text).unwrap();
        let result = resolve(text, text, text, &review, &BTreeMap::new()).unwrap();
        assert_eq!(result.content, text);
        assert!(!result.syntax_verified && !result.ownership_verified && !result.runtime_verified);
        assert!(result.fragments.iter().all(|p| p.input == Input::Incoming));
    }
    assert_eq!(automatic("a", "a\n", "a"), "a\n");
    assert_eq!(automatic("a\n", "a", "a\n"), "a");
    assert_eq!(automatic("a", "é", "é"), "é");
}

#[test]
fn insertions_inside_a_document_keep_their_real_base_coordinates_and_branch_coverage() {
    let base = "header\nend\n";
    let private = "header\nleft\nend\n";
    let incoming = "header\nright\nend\n";
    let review = prepare(base, private, incoming).unwrap();
    validate_review(base, private, incoming, &review).unwrap();
    let (id, region) = review
        .regions
        .iter()
        .enumerate()
        .find(|(_, r)| r.automatic.is_none())
        .unwrap();
    assert_eq!(region.base, 7..7);
    let choices = [(id, Choice::IncomingThenPrivate)].into();
    let result = resolve(base, private, incoming, &review, &choices).unwrap();
    assert_eq!(result.content, "header\nright\nleft\nend\n");
    validate_resolution(base, private, incoming, &review, &choices, &result).unwrap();
    let mut bad = result;
    bad.content.push_str("unselected");
    bad.sha256 = source_hash(&bad.content);
    assert!(validate_resolution(base, private, incoming, &review, &choices, &bad).is_err());
}

#[test]
fn exhaustive_small_three_way_inputs_keep_every_branch_byte_and_explicit_result_bound() {
    let inputs = [
        "", "a", "b", "ab", "ba", "aa", "bb", "\n", "a\n", "b\n", "\na", "a\nb\n", "a\n\na\n", "é",
        "e\u{301}",
    ];
    for base in inputs {
        for private in inputs {
            for incoming in inputs {
                let review = prepare(base, private, incoming)
                    .unwrap_or_else(|e| panic!("{base:?}, {private:?}, {incoming:?}: {e}"));
                validate_review(base, private, incoming, &review).unwrap();
                let choices = review
                    .regions
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.automatic.is_none())
                    .map(|(id, _)| (id, Choice::Base))
                    .collect();
                let result = resolve(base, private, incoming, &review, &choices).unwrap();
                validate_resolution(base, private, incoming, &review, &choices, &result).unwrap();
            }
        }
    }
    assert_eq!(
        automatic("value=1", "value=2", "value=1-- comment"),
        "value=2-- comment"
    );
    assert_eq!(automatic("a", "", "a\n"), "\n");
}

#[test]
fn unchanged_incoming_loader_bytes_keep_their_provenance_inside_a_privately_changed_line() {
    let base = "local r=require('part');hl.monitor{output='DP-1',transform=1}\n";
    let private = base.replace("transform=1", "transform=3");
    let incoming = base.replace("DP-1", "DP-2");
    let review = prepare(base, &private, &incoming).unwrap();
    let result = resolve(base, &private, &incoming, &review, &BTreeMap::new()).unwrap();
    assert_eq!(
        result.content,
        incoming.replace("transform=1", "transform=3")
    );
    let call = 0..base.find(';').unwrap();
    assert!(result.fragments.iter().any(|p| p.input == Input::Incoming
        && p.span.start <= call.start
        && p.span.end >= call.end));
    let mut forged = review.clone();
    forged.regions[0].incoming[0].input = Input::Private;
    assert!(validate_review(base, &private, &incoming, &forged).is_err());
}
#[test]
fn large_sparse_changes_use_bounded_linear_frontiers_and_preserve_every_unrelated_line() {
    let base = (0..4000)
        .map(|n| format!("-- row {n}\nhl.config{{general={{border_size=1}}}}\n"))
        .collect::<String>();
    let private = base.replacen(
        "-- row 1000\nhl.config{general={border_size=1}}",
        "-- row 1000\nhl.config{general={border_size=2}}",
        1,
    );
    let incoming = base.replacen(
        "-- row 3000\nhl.config{general={border_size=1}}",
        "-- row 3000\nhl.config{general={border_size=3}}",
        1,
    );
    let review = prepare(&base, &private, &incoming).unwrap();
    let result = resolve(&base, &private, &incoming, &review, &BTreeMap::new()).unwrap();
    assert_eq!(
        result.content,
        private.replacen(
            "-- row 3000\nhl.config{general={border_size=1}}",
            "-- row 3000\nhl.config{general={border_size=3}}",
            1
        )
    );
    assert!(review.peak_frontier_cells < 40000);
    assert!(review.work < MAX_WORK);
}

#[test]
fn deterministic_multisymbol_scripts_match_the_independent_distance_oracle() {
    let mut state = 0x9182_4321_u32;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for _ in 0..500 {
        let n = (next() % 41) as usize;
        let m = (next() % 41) as usize;
        let a = (0..n).map(|_| next() % 7).collect::<Vec<_>>();
        let b = (0..m).map(|_| next() % 7).collect::<Vec<_>>();
        let script = diff::script(&a, &b, &mut Budget::new()).unwrap();
        assert_eq!(script.distance, n + m - 2 * lcs(&a, &b, None));
        for (old, new) in &script.equals {
            for offset in 0..old.len() {
                let edge = (old.start + offset, new.start + offset);
                assert_eq!(
                    diff::alternative(&a, &b, edge, script.distance, &mut Budget::new()).unwrap(),
                    lcs(&a, &b, Some(edge)) == lcs(&a, &b, None)
                );
            }
        }
    }
}
#[test]
fn choosing_both_insertions_cannot_bypass_the_result_limit_or_frame_pins() {
    let private = "a".repeat(MAX_DOCUMENT_BYTES / 2 + 1);
    let incoming = "b".repeat(MAX_DOCUMENT_BYTES / 2 + 1);
    let review = prepare("", &private, &incoming).unwrap();
    assert!(resolve(
        "",
        &private,
        &incoming,
        &review,
        &[(0, Choice::PrivateThenIncoming)].into()
    )
    .is_err());
    let mut result = resolve(
        "",
        &private,
        &incoming,
        &review,
        &[(0, Choice::Private)].into(),
    )
    .unwrap();
    result.syntax_verified = true;
    assert!(validate_resolution(
        "",
        &private,
        &incoming,
        &review,
        &[(0, Choice::Private)].into(),
        &result
    )
    .is_err());
}
