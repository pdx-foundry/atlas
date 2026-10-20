use super::*;
use crate::ledger::inventory;

const LEDGER_SHA256: &str = "ledger-sha256";

fn fixture() -> (Ledger, coverage::Report) {
    let ledger = inventory(
        &[(
            "traditions.cwt".into(),
            "types = { type[tradition] = { path = \"game/common/traditions\" } }\n\
             tradition = { unlocks_agenda = int }\n"
                .into(),
        )]
        .into(),
    );
    let report = coverage::evaluate(&ledger, None, None).unwrap();

    (ledger, report)
}

fn atlas_owned<'a>(ledger: &'a Ledger, property: &str) -> &'a Claim {
    ledger
        .claims
        .iter()
        .find(|claim| claim.property == property && claim.owner.atlas_owned())
        .unwrap()
}

fn baseline(ledger: &Ledger, claims: &[&str]) -> Baseline {
    Baseline {
        config_sha256: ledger.config_sha256.clone(),
        ledger_sha256: LEDGER_SHA256.into(),
        covered: claims.len(),
        total: 0,
        observed_live: false,
        files: [(
            "traditions.cwt".to_owned(),
            claims.iter().map(|claim| (*claim).to_owned()).collect(),
        )]
        .into(),
    }
}

fn comparison(config_sha256: &str, snapshot_sha256: &str, different: &Claim) -> ComparisonSummary {
    let entry = |status: &str| ComparisonEntry {
        status: status.into(),
        claim: None,
        config_answer: None,
        atlas_answers: Vec::new(),
    };

    ComparisonSummary {
        config_sha256: config_sha256.into(),
        snapshot_sha256: snapshot_sha256.into(),
        entries: vec![
            entry("same"),
            entry("same"),
            ComparisonEntry {
                claim: Some(different.id.clone()),
                config_answer: Some("int".into()),
                atlas_answers: vec![
                    serde_json::json!("Accumulate"),
                    serde_json::json!({"form": "number", "reader": "a98"}),
                    serde_json::json!({"form": "string", "values": ["a"]}),
                ],
                ..entry("different")
            },
        ],
    }
}

fn registries(config_sha256: &str, build: &str) -> RegistryJoins {
    RegistryJoins {
        config_sha256: config_sha256.into(),
        native_source: RegistrySource {
            build: build.into(),
        },
        rows: vec![RegistryRow {
            type_name: "type[tradition]".into(),
            file: "traditions.cwt".into(),
            join: "named".into(),
        }],
    }
}

#[test]
fn a_claim_the_baseline_covers_and_the_run_does_not_is_lost_by_name() {
    let (ledger, mut report) = fixture();
    let lost = atlas_owned(&ledger, "value_form").id.clone();
    let gained = atlas_owned(&ledger, "loader_path").id.clone();
    report
        .claims
        .iter_mut()
        .find(|assessment| assessment.claim == gained)
        .unwrap()
        .covered = true;
    let baseline = baseline(&ledger, &[&lost, "question:removed:1"]);

    let page = page_data(&ledger, LEDGER_SHA256, &report, Some(&baseline), None, None);

    assert_eq!(page.lost.len(), 1);
    assert_eq!(page.lost[0].claim, lost);
    assert_eq!(page.lost[0].file, "traditions.cwt");
    assert_eq!(page.lost[0].property, "value_form");
    assert!(!page.lost[0].reason.is_empty());
    assert_eq!(page.gained, 1);
    assert_eq!(page.missing_from_ledger, 1);
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);

    let other = page
        .areas
        .iter()
        .find(|row| row.name == OTHER_AREA)
        .unwrap();
    assert_eq!(other.total, report.totals.atlas_owned.total);
    assert_eq!(other.baseline_covered, Some(1));

    let [tradition] = page.gate_types.as_slice() else {
        panic!("one gate type: {:?}", page.gate_types);
    };
    assert_eq!(tradition.name, "type[tradition]");
    assert_eq!(
        tradition.total,
        report.by_type["traditions.cwt"].types["type[tradition]"]
            .atlas_owned
            .total
    );
    assert_eq!(tradition.baseline_covered, Some(1));
}

#[test]
fn without_a_baseline_no_change_is_measured() {
    let (ledger, report) = fixture();

    let page = page_data(&ledger, LEDGER_SHA256, &report, None, None, None);

    assert!(page.baseline.is_none());
    assert!(page.lost.is_empty());
    assert!(page.areas.iter().all(|row| row.baseline_covered.is_none()));
    assert!(
        page.gate_types
            .iter()
            .all(|row| row.baseline_covered.is_none())
    );
}

#[test]
fn a_baseline_of_another_ledger_warns() {
    let (ledger, report) = fixture();
    let baseline = baseline(&ledger, &[]);

    let page = page_data(
        &ledger,
        "other-ledger",
        &report,
        Some(&baseline),
        None,
        None,
    );

    assert_eq!(page.warnings.len(), 1);
    assert!(page.warnings[0].contains("another ledger"));
}

#[test]
fn reports_of_other_inputs_are_left_out_with_a_warning() {
    let (ledger, mut report) = fixture();
    report.snapshot_sha256 = Some("snapshot".into());
    report.target = Some("build".into());
    let config = ledger.config_sha256.clone();
    let different = atlas_owned(&ledger, "value_form");

    let matching = page_data(
        &ledger,
        LEDGER_SHA256,
        &report,
        None,
        Some(&comparison(&config, "snapshot", different)),
        Some(&registries(&config, "build")),
    );
    let stale = page_data(
        &ledger,
        LEDGER_SHA256,
        &report,
        None,
        Some(&comparison(&config, "other-snapshot", different)),
        Some(&registries(&config, "other-build")),
    );

    assert_eq!(
        matching.agreement,
        Some([("different".into(), 1), ("same".into(), 2)].into())
    );
    let [disagreement] = matching.disagreements.as_slice() else {
        panic!("one disagreement: {:?}", matching.disagreements);
    };
    assert_eq!(disagreement.property, "value_form");
    assert_eq!(disagreement.config_answer, "int");
    assert_eq!(
        disagreement.atlas_answers,
        [
            "Accumulate",
            "number",
            r#"{"form":"string","values":["a"]}"#
        ]
    );
    assert!(stale.disagreements.is_empty());
    assert_eq!(matching.gate_types[0].join.as_deref(), Some("named"));
    assert!(matching.warnings.is_empty(), "{:?}", matching.warnings);
    assert_eq!(stale.agreement, None);
    assert_eq!(stale.gate_types[0].join, None);
    assert_eq!(stale.warnings.len(), 2);
}

/// A ledger of two config files and a report that covers every Atlas-owned claim but `uncovered`.
fn two_files(uncovered: &[&str]) -> (Ledger, coverage::Report) {
    let ledger = inventory(
        &[
            (
                "traditions.cwt".into(),
                "types = { type[tradition] = { path = \"game/common/traditions\" } }\n\
                 tradition = { unlocks_agenda = int }\n"
                    .into(),
            ),
            (
                "common/relics.cwt".into(),
                "types = { type[relic] = { path = \"game/common/relics\" } }\n\
                 relic = { score = int }\n"
                    .into(),
            ),
        ]
        .into(),
    );
    let mut report = coverage::evaluate(&ledger, None, None).unwrap();
    let atlas_owned: BTreeSet<&str> = ledger
        .claims
        .iter()
        .filter(|claim| claim.owner.atlas_owned())
        .map(|claim| claim.id.as_str())
        .collect();

    for assessment in &mut report.claims {
        assessment.covered = atlas_owned.contains(assessment.claim.as_str())
            && !uncovered.contains(&assessment.claim.as_str());
    }

    report.totals.atlas_owned.covered = report
        .claims
        .iter()
        .filter(|assessment| assessment.covered)
        .count();

    (ledger, report)
}

fn claim_of(ledger: &Ledger, file: &str, property: &str) -> String {
    ledger
        .claims
        .iter()
        .find(|claim| claim.file == file && claim.property == property && claim.owner.atlas_owned())
        .unwrap()
        .id
        .clone()
}

#[test]
fn baseline_text_round_trips() {
    let (ledger, report) = two_files(&[]);

    let baseline = Baseline::of(&ledger, LEDGER_SHA256, &report, true);
    let texts = baseline.render();

    assert_eq!(baseline.files.len(), 2);
    assert_eq!(
        texts.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "common%2Frelics.cwt.txt",
            BASELINE_INDEX,
            "traditions.cwt.txt"
        ]
    );
    assert_eq!(Baseline::parse(&texts), Ok(baseline));
}

#[test]
fn the_split_baseline_reconstructs_the_page_of_the_run() {
    let (ledger, _) = two_files(&[]);
    let lost = claim_of(&ledger, "common/relics.cwt", "value_form");
    let gained = claim_of(&ledger, "traditions.cwt", "value_form");
    let (_, earlier) = two_files(&[&gained]);
    let (_, later) = two_files(&[&lost]);
    let whole = Baseline::of(&ledger, LEDGER_SHA256, &earlier, true);
    let loaded = Baseline::parse(&whole.render()).unwrap();

    let page = |baseline: &Baseline| {
        let page = page_data(&ledger, "other-ledger", &later, Some(baseline), None, None);
        serde_json::to_value(page).unwrap()
    };

    assert_eq!(page(&loaded), page(&whole));
    assert_eq!(page(&loaded)["gained"], 1);
    assert_eq!(page(&loaded)["lost"][0]["claim"], lost.as_str());
    assert_eq!(page(&loaded)["warnings"].as_array().unwrap().len(), 1);
}

#[test]
fn a_change_to_one_files_claims_changes_only_its_shard_and_the_index() {
    let (ledger, report) = two_files(&[]);
    let relic = claim_of(&ledger, "common/relics.cwt", "value_form");
    let (_, changed) = two_files(&[&relic]);

    let before = Baseline::of(&ledger, LEDGER_SHA256, &report, true).render();
    let after = Baseline::of(&ledger, LEDGER_SHA256, &changed, true).render();
    let differ: Vec<_> = before
        .keys()
        .filter(|path| before.get(*path) != after.get(*path))
        .map(String::as_str)
        .collect();

    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>()
    );
    assert_eq!(differ, ["common%2Frelics.cwt.txt", BASELINE_INDEX]);
}

#[test]
fn shard_names_are_flat_and_distinct() {
    assert_eq!(
        shard_path("common/traditions.cwt"),
        "common%2Ftraditions.cwt.txt"
    );
    assert_ne!(
        shard_path("foo.cwt.txt/bar.cwt"),
        shard_path("foo.cwt.txt%2Fbar.cwt")
    );
    assert!(!shard_path("foo.cwt.txt/bar.cwt").contains('/'));
}

#[test]
fn an_incomplete_or_inconsistent_baseline_is_refused() {
    let (ledger, report) = two_files(&[]);
    let texts = Baseline::of(&ledger, LEDGER_SHA256, &report, true).render();
    let edited = |edit: &dyn Fn(&mut BTreeMap<String, String>)| {
        let mut texts = texts.clone();
        edit(&mut texts);
        Baseline::parse(&texts).unwrap_err()
    };

    assert!(
        edited(&|texts| {
            texts.remove("traditions.cwt.txt");
        })
        .contains("missing")
    );
    assert!(
        edited(&|texts| {
            texts.insert("stale.cwt.txt".into(), "question:x:1\n".into());
        })
        .contains("not in the index")
    );
    assert!(
        edited(&|texts| {
            texts
                .get_mut("traditions.cwt.txt")
                .unwrap()
                .push_str("question:x:1\n");
        })
        .contains("holds")
    );
    assert!(
        edited(&|texts| {
            let relic = texts["common%2Frelics.cwt.txt"]
                .lines()
                .next()
                .unwrap()
                .to_owned();
            let tradition = texts["traditions.cwt.txt"]
                .lines()
                .next()
                .unwrap()
                .to_owned();
            let shard = texts.get_mut("traditions.cwt.txt").unwrap();
            *shard = shard.replacen(&tradition, &relic, 1);
        })
        .contains("more than one shard")
    );
    assert!(
        edited(&|texts| {
            let index = texts.get_mut(BASELINE_INDEX).unwrap();
            *index = index.replace("observed_live true", "observed_live yes");
        })
        .contains("not a bool")
    );
    assert!(
        edited(&|texts| {
            let index = &texts[BASELINE_INDEX];
            let covered = index
                .lines()
                .find_map(|line| line.strip_prefix("atlas_owned_covered "))
                .unwrap()
                .to_owned();
            let index = index.replace(
                &format!("atlas_owned_covered {covered}"),
                "atlas_owned_covered 0",
            );
            texts.insert(BASELINE_INDEX.into(), index);
        })
        .contains("not the 0 covered")
    );
    assert!(Baseline::parse(&BTreeMap::new()).is_err());
}

#[test]
fn source_text_cannot_close_the_data_script() {
    let (ledger, report) = fixture();
    let mut page = page_data(&ledger, LEDGER_SHA256, &report, None, None, None);
    page.warnings.push("</script><b>markup</b>".into());

    let html = render_html(&page);

    assert!(!html.contains(DATA_PLACEHOLDER));
    assert_eq!(html.matches("</script>").count(), 2);
    assert!(html.contains("\\u003c/script>\\u003cb>markup"));
}

#[test]
fn gate_types_put_the_lowest_share_first() {
    let row = |name: &str, covered, total| Row {
        name: name.into(),
        covered,
        total,
        ..Row::default()
    };
    let mut rows = [
        row("empty", 0, 0),
        row("half", 5, 10),
        row("small-gap", 0, 2),
        row("large-gap", 0, 9),
        row("tenth", 1, 10),
    ];

    rows.sort_by(least_covered_first);

    let names: Vec<_> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["large-gap", "small-gap", "tenth", "half", "empty"]);
}
