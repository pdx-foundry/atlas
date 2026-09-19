use pdx_atlas::{
    coverage, ledger,
    provenance::{self, Corpus, Match, Origin, parse_comments},
    report,
};
use std::collections::BTreeMap;

fn sources(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(file, text)| (file.to_string(), text.to_string()))
        .collect()
}
#[test]
fn comments_bind_to_keys_without_crossing_groups_or_reading_quoted_hashes() {
    let text = "\u{feff}N = {\r\n # shared documentation\r\n A = 1\r\n B = 2 # B only\r\n\r\n C = \"not # a comment\"\r\n # next entry\r\n D = { x = 1 } # whole block\r\n E = { # opening block\r\n  y = 2\r\n }\r\n}\r\n";
    let (parsed, errors) = parse_comments("common/defines/00.txt", text);
    assert!(errors.is_empty(), "{errors:?}");
    for key in ["A", "B"] {
        assert!(
            parsed
                .iter()
                .any(|s| s.key == ["N", key] && s.text == "shared documentation")
        );
    }
    assert!(!parsed.iter().any(|s| s.key == ["N", "C"]));
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["N", "B"] && s.text == "B only" && s.line == 4)
    );
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["N", "D"] && s.text == "whole block")
    );
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["N", "E"] && s.text == "opening block")
    );
    assert!(!parsed.iter().any(|s| s.text.contains("not #")));
}
#[test]
fn documentation_guides_and_empty_example_blocks_keep_locations() {
    let text = "# hostility -> Will this empire attack its neighbors?\n# field = yes # The first line\n#             # and the second line\n\n# example = {\n# condition = {\n#   A trigger for the condition.\n# }\n# }\n";
    let (parsed, _) = parse_comments("common/things/00_example.txt", text);
    assert!(parsed.iter().any(|s| s.key == ["hostility"]
        && s.text == "Will this empire attack its neighbors?"
        && s.line == 1));
    assert!(parsed.iter().any(|s| s.key == ["field"]
        && s.text == "The first line\nand the second line"
        && s.line == 2
        && s.end_line == 3));
    assert!(parsed.iter().any(|s| s.key == ["example", "condition"]
        && s.text == "A trigger for the condition."
        && s.line == 7));
}
#[test]
fn all_entries_are_tagged_and_near_matches_are_separate_from_exact_and_authored() {
    let config = sources(&[
        (
            "effects.cwt",
            "### Changes the amount of energy stored by the country\nalias[effect:add_energy] = int\n### Changes the total amount of energy stored by the country\nalias[effect:add_energy] = float\n### Changes the amount of energy stored by the country\nalias[effect:wrong_command] = int\n### Entirely authored explanation\nalias[effect:custom] = bool\n",
        ),
        (
            "on_actions.cwt",
            "on_actions = {\n ### Fires when a game begins\n on_start\n}\n",
        ),
        (
            "common/things.cwt",
            "types = { type[thing] = { path = game/common/things } }\nthing = {\n ### A documented\n #### field\n value = int\n}\n",
        ),
    ]);
    let corpus = Corpus::from_sources(
        &sources(&[
            (
                "common/on_actions/00.txt",
                "# Fires when a game begins\non_start = {}\n",
            ),
            (
                "common/things/00.txt",
                "example = {\n value = 2 # A documented field\n}\n",
            ),
        ]),
        &sources(&[(
            "effects.log",
            "== EFFECT DOCUMENTATION ==\nadd_energy - Changes the amount of energy stored by the country\nadd_energy = 10\nSupported Scopes: country\n",
        )]),
    );
    let mut ledger = ledger::inventory(&config);
    let ids: Vec<_> = ledger.claims.iter().map(|c| c.id.clone()).collect();
    let measured = provenance::annotate(&mut ledger, &corpus);
    assert_eq!(
        ids,
        ledger
            .claims
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(measured.totals.entries, 6);
    assert_eq!(measured.totals.exact, 3);
    assert_eq!(measured.totals.rewritten, 1);
    assert_eq!(measured.totals.authored, 2);
    assert_eq!(measured.by_family["type_schemas"].lines, 2);
    for claim in ledger
        .claims
        .iter()
        .filter(|c| c.property == "documentation")
    {
        let attribution = claim.provenance.as_ref().unwrap();
        if attribution.origin == Origin::Authored {
            assert_eq!(claim.owner, ledger::Owner::AuthoredText);
            assert!(attribution.sources.is_empty());
            assert!(measured.authored.contains(&claim.id));
        } else {
            assert!(!attribution.sources.is_empty());
            assert!(
                attribution
                    .sources
                    .iter()
                    .all(|s| s.sha256.len() == 64 && s.line > 0 && s.end_line >= s.line)
            );
        }
        assert_eq!(
            claim.provisional_owner,
            attribution.comparison == Match::Rewritten
        );
    }
    assert_eq!(
        coverage::evaluate(&ledger, None)
            .unwrap()
            .totals
            .atlas_owned
            .covered,
        0
    );
    let first = report::json_bytes(&ledger).unwrap();
    let second = provenance::annotate(&mut ledger, &corpus);
    assert_eq!(first, report::json_bytes(&ledger).unwrap());
    assert_eq!(
        report::json_bytes(&measured).unwrap(),
        report::json_bytes(&second).unwrap()
    );
}
#[test]
fn define_namespaces_and_content_families_cannot_borrow_matches() {
    let config = sources(&[
        (
            "common/defines/00_defines.cwt",
            "defines = { Wrong = {\n ### shared words\n VALUE = int\n} }\n",
        ),
        (
            "common/things.cwt",
            "thing = {\n ### shared words\n field = int\n}\n",
        ),
    ]);
    let corpus = Corpus::from_sources(
        &sources(&[
            (
                "common/defines/00.txt",
                "Right = { VALUE = 1 # shared words\n}\n",
            ),
            (
                "common/other/00.txt",
                "obj = { field = 1 # shared words\n}\n",
            ),
        ]),
        &BTreeMap::new(),
    );
    let mut ledger = ledger::inventory(&config);
    assert_eq!(
        provenance::annotate(&mut ledger, &corpus).totals.authored,
        2
    );
}
#[test]
fn duplicate_locations_are_retained_and_source_changes_alter_identity() {
    let config = sources(&[(
        "common/things.cwt",
        "thing = {\n ### shared words\n field = int\n}\n",
    )]);
    let mut content = sources(&[
        ("common/things/a.txt", "a = { field = 1 # shared words\n}\n"),
        ("common/things/b.txt", "b = { field = 2 # shared words\n}\n"),
    ]);
    let mut ledger = ledger::inventory(&config);
    let first = provenance::annotate(
        &mut ledger,
        &Corpus::from_sources(&content, &BTreeMap::new()),
    );
    assert_eq!(
        ledger
            .claims
            .iter()
            .find_map(|c| c.provenance.as_ref())
            .unwrap()
            .sources
            .len(),
        2
    );
    content.get_mut("common/things/a.txt").unwrap().push('\n');
    let second = provenance::annotate(
        &mut ledger,
        &Corpus::from_sources(&content, &BTreeMap::new()),
    );
    assert_ne!(first.inputs, second.inputs);
}
#[test]
fn parser_failures_are_reported_without_discarding_explicit_comment_sources() {
    let corpus = Corpus::from_sources(
        &sources(&[(
            "common/things/broken.txt",
            "# field: source text\nbroken = \"unterminated",
        )]),
        &BTreeMap::new(),
    );
    let mut ledger = ledger::inventory(&sources(&[(
        "common/things.cwt",
        "thing = {\n ### source text\n field = int\n}\n",
    )]));
    let report = provenance::annotate(&mut ledger, &corpus);
    assert_eq!(report.totals.exact, 1);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.inputs.len(), 1);
}

#[test]
fn commented_quoted_hashes_are_not_documentation_delimiters() {
    let (parsed, errors) = parse_comments(
        "common/things/guide.txt",
        "# field = \"not # prose\" # Actual documentation\n",
    );
    assert!(errors.is_empty());
    assert!(
        parsed
            .iter()
            .any(|source| source.key == ["field"] && source.text == "Actual documentation")
    );
    assert!(!parsed.iter().any(|source| source.text.contains("not #")));
}
