use pdx_atlas::{coverage::*, ledger::*};
use serde_json::json;
fn fixture() -> (Ledger, Snapshot) {
    let ledger = inventory(&[("example.cwt".into(), "x = bool".into())].into());
    let claim = ledger
        .claims
        .iter()
        .find(|c| c.property == "value_form")
        .unwrap();
    let snapshot = Snapshot {
        kind: "atlas_coverage".into(),
        format_version: 1,
        snapshot_id: "test-input".into(),
        target: "target-sha256".into(),
        gaps: vec![],
        answers: vec![Answer {
            question: claim.question.clone(),
            conditions: claim.conditions.clone(),
            value: json!("bool"),
            status: Status::Supported,
            evidence: vec![Evidence {
                id: "evidence:field".into(),
                method: "qualified-reader".into(),
                target: "target-sha256".into(),
                qualified: true,
                origin: Origin::Engine,
            }],
        }],
    };
    (ledger, snapshot)
}
fn score(ledger: &Ledger, snapshot: &Snapshot) -> Report {
    evaluate(ledger, Some(&serde_json::to_vec(snapshot).unwrap())).unwrap()
}
#[test]
fn disagreements_and_agreements_receive_the_same_credit() {
    let (ledger, mut snapshot) = fixture();
    let same = score(&ledger, &snapshot);
    snapshot.answers[0].value = json!("integer");
    let different = score(&ledger, &snapshot);
    assert_eq!(same.totals.atlas_owned.covered, 1);
    assert_eq!(different.totals.atlas_owned.covered, 1);
    assert_eq!(
        same.totals.atlas_owned.percent,
        different.totals.atlas_owned.percent
    );
}
#[test]
fn negative_existence_answer_does_not_cover_other_properties() {
    let (ledger, mut snapshot) = fixture();
    snapshot.answers[0].question = ledger
        .claims
        .iter()
        .find(|c| c.property == "field_existence")
        .unwrap()
        .question
        .clone();
    snapshot.answers[0].value = json!({"exists":false});
    let report = score(&ledger, &snapshot);
    assert_eq!(report.totals.atlas_owned.covered, 1);
    let form = ledger
        .claims
        .iter()
        .find(|c| c.property == "value_form")
        .unwrap();
    assert!(
        !report
            .claims
            .iter()
            .find(|a| a.claim == form.id)
            .unwrap()
            .covered
    );
}
#[test]
fn unsupported_or_inapplicable_material_never_inflates_coverage() {
    let (ledger, original) = fixture();
    for status in [
        Status::Unknown,
        Status::Untested,
        Status::Partial,
        Status::Unsupported,
        Status::Conflicted,
    ] {
        let mut s = original.clone();
        s.answers[0].status = status;
        assert_eq!(score(&ledger, &s).totals.atlas_owned.covered, 0);
    }
    for variant in 0..6 {
        let mut s = original.clone();
        match variant {
            0 => s.answers[0].evidence.clear(),
            1 => s.answers[0].evidence[0].origin = Origin::Synthetic,
            2 => s.answers[0].evidence[0].qualified = false,
            3 => s.answers[0].evidence[0].target = "different-build".into(),
            4 => s.answers[0].conditions.push("narrower-condition".into()),
            _ => s.answers[0].evidence[0].origin = Origin::Authored,
        }
        assert_eq!(
            score(&ledger, &s).totals.atlas_owned.covered,
            0,
            "variant {variant}"
        );
    }
}
#[test]
fn conflicts_and_gaps_prevent_complete_credit() {
    let (ledger, mut snapshot) = fixture();
    let mut other = snapshot.answers[0].clone();
    other.value = json!("another answer");
    snapshot.answers.push(other);
    assert_eq!(score(&ledger, &snapshot).totals.atlas_owned.covered, 0);
    snapshot.answers.pop();
    snapshot.gaps.push(Gap {
        question: snapshot.answers[0].question.clone(),
        conditions: vec![],
        reason: "unknown context".into(),
    });
    assert_eq!(score(&ledger, &snapshot).totals.atlas_owned.covered, 0);
}
#[test]
fn registry_names_are_observations_without_rule_credit() {
    let (ledger, _) = fixture();
    let input = json!({"queries":{"traditions":{"Ok":{"native":{"registeredItems":[{"key":"x"}],"activation":"Demonstrated","completion":"Complete","limits":["items only"]},"gaps":["no rules"]}}}});
    let report = evaluate(&ledger, Some(&serde_json::to_vec(&input).unwrap())).unwrap();
    assert_eq!(report.totals.atlas_owned.covered, 0);
    assert_eq!(report.registry_observations[0]["observed_items"], 1);
}
#[test]
fn empty_denominators_and_unknown_formats_are_explicit() {
    let ledger = inventory(&[("empty.cwt".into(), "#empty".into())].into());
    assert_eq!(
        evaluate(&ledger, None).unwrap().totals.atlas_owned.percent,
        None
    );
    assert!(evaluate(&ledger, Some(b"{}")).is_err());
}
#[test]
fn per_file_and_owner_totals_reconcile() {
    let (ledger, snapshot) = fixture();
    let report = score(&ledger, &snapshot);
    assert_eq!(
        report
            .by_file
            .values()
            .map(|t| t.all_claims.total)
            .sum::<usize>(),
        report.totals.all_claims.total
    );
    assert_eq!(
        report
            .totals
            .by_owner
            .values()
            .map(|f| f.covered)
            .sum::<usize>(),
        report.totals.all_claims.covered
    );
    assert_eq!(report.totals.by_owner.len(), 4);
}

#[test]
fn policy_and_authored_text_do_not_enter_headline_denominator() {
    let ledger = inventory(
        &[(
            "rules.cwt".into(),
            "### Written explanation\n## severity = warning\nx = bool".into(),
        )]
        .into(),
    );
    let report = evaluate(&ledger, None).unwrap();
    assert_eq!(report.totals.atlas_owned.total, 2);
    assert_eq!(report.totals.all_claims.total, 4);
    assert_eq!(report.totals.by_owner[&Owner::ConsumerPolicy].total, 1);
    assert_eq!(report.totals.by_owner[&Owner::AuthoredText].total, 1);
}
#[test]
fn duplicate_evidence_does_not_add_credit_and_atlas_only_answers_do_not_change_denominator() {
    let (ledger, mut s) = fixture();
    s.answers.push(s.answers[0].clone());
    let mut extra = s.answers[0].clone();
    extra.question = "question:atlas-only".into();
    s.answers.push(extra);
    let report = score(&ledger, &s);
    assert_eq!(report.totals.atlas_owned.covered, 1);
    assert_eq!(report.totals.atlas_owned.total, 2);
    assert_eq!(report.atlas_only_questions, vec!["question:atlas-only"]);
}
