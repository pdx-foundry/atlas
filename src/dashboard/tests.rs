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
    let report = coverage::evaluate(&ledger, None).unwrap();

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
        claims: claims.iter().map(|claim| (*claim).to_owned()).collect(),
    }
}

fn comparison(config_sha256: &str, snapshot_sha256: &str) -> ComparisonSummary {
    ComparisonSummary {
        config_sha256: config_sha256.into(),
        snapshot_sha256: snapshot_sha256.into(),
        entries: ["same", "same", "different"]
            .into_iter()
            .map(|status| ComparisonEntry {
                status: status.into(),
            })
            .collect(),
    }
}

fn registries(config_sha256: &str) -> RegistryJoins {
    RegistryJoins {
        config_sha256: config_sha256.into(),
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
    let config = ledger.config_sha256.clone();

    let matching = page_data(
        &ledger,
        LEDGER_SHA256,
        &report,
        None,
        Some(&comparison(&config, "snapshot")),
        Some(&registries(&config)),
    );
    let stale = page_data(
        &ledger,
        LEDGER_SHA256,
        &report,
        None,
        Some(&comparison(&config, "other-snapshot")),
        Some(&registries("other-config")),
    );

    assert_eq!(
        matching.agreement,
        Some([("different".into(), 1), ("same".into(), 2)].into())
    );
    assert_eq!(matching.gate_types[0].join.as_deref(), Some("named"));
    assert!(matching.warnings.is_empty(), "{:?}", matching.warnings);
    assert_eq!(stale.agreement, None);
    assert_eq!(stale.gate_types[0].join, None);
    assert_eq!(stale.warnings.len(), 2);
}

#[test]
fn baseline_text_round_trips() {
    let (ledger, mut report) = fixture();
    report.claims[0].covered = true;
    report.totals.atlas_owned.covered = 1;

    let baseline = Baseline::of(&ledger, LEDGER_SHA256, &report);

    assert_eq!(baseline.claims.len(), 1);
    assert_eq!(Baseline::parse(&baseline.render()), Ok(baseline));
    assert!(Baseline::parse("config_sha256 x\n").is_err());
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
