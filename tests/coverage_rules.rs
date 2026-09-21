use pdx_atlas::{coverage, extraction, ledger, snapshot};
use pdx_native::{Basis, GameOptions, Native};
use serde_json::json;
use std::{collections::BTreeMap, path::Path, process::Command};

fn ledger() -> ledger::Ledger {
    let source = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
    }
}
tradition = {
    unlocks_agenda = scalar
}
"#;
    ledger::inventory(&BTreeMap::from([("traditions.cwt".into(), source.into())]))
}

async fn recorded_snapshot() -> snapshot::Snapshot {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m45");
    let native = Native::from_recorded_answers(path).unwrap();
    let extraction =
        extraction::collect(&native, || GameOptions::new(Command::new("unused"))).await;
    snapshot::assemble(&extraction).unwrap()
}

fn qualify_as_live(snapshot: &mut snapshot::Snapshot) {
    let mut keys = BTreeMap::new();
    let sources = std::mem::take(&mut snapshot.sources);
    for (old, mut source) in sources {
        source.basis = Basis::LiveObservation;
        let new = format!("{}@LiveObservation", source.method);
        keys.insert(old, new.clone());
        snapshot.sources.insert(new, source);
    }
    for rule in &mut snapshot.rules {
        for evidence in &mut rule.evidence {
            evidence.source = keys[&evidence.source].clone();
        }
    }
    for gap in &mut snapshot.gaps {
        for evidence in &mut gap.evidence {
            evidence.source = keys[&evidence.source].clone();
        }
    }
}

#[tokio::test]
async fn rule_snapshot_credit_requires_live_basis_and_no_applicable_gap() {
    let snapshot = recorded_snapshot().await;
    let ledger = ledger();
    let recorded =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();
    assert_eq!(recorded.totals.atlas_owned.covered, 0);

    let mut live = snapshot;
    qualify_as_live(&mut live);
    let report = coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&live).unwrap())).unwrap();
    assert!(report.totals.atlas_owned.covered > 0);
    let field = ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "field_existence" && claim.subject == ["tradition", "unlocks_agenda"]
        })
        .unwrap();
    let form = ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "value_form" && claim.subject == ["tradition", "unlocks_agenda"]
        })
        .unwrap();
    assert!(
        report
            .claims
            .iter()
            .find(|claim| claim.claim == field.id)
            .unwrap()
            .covered
    );
    assert!(
        !report
            .claims
            .iter()
            .find(|claim| claim.claim == form.id)
            .unwrap()
            .covered
    );
    assert!(
        report
            .atlas_only_questions
            .iter()
            .any(|question| question.contains("potential"))
    );
}

#[tokio::test]
async fn invalid_rule_snapshot_is_rejected() {
    let mut snapshot = recorded_snapshot().await;
    snapshot.contract_version = 2;
    assert!(
        coverage::evaluate(&ledger(), Some(&snapshot::json_bytes(&snapshot).unwrap())).is_err()
    );
    snapshot.contract_version = 1;
    snapshot.schema_dialect = "https://json-schema.org/draft-07/schema".into();
    assert!(
        coverage::evaluate(&ledger(), Some(&snapshot::json_bytes(&snapshot).unwrap())).is_err()
    );
    snapshot.schema_dialect = "https://json-schema.org/draft/2020-12/schema".into();
    snapshot.schemas.dialect = "https://json-schema.org/draft-07/schema".into();
    assert!(
        coverage::evaluate(&ledger(), Some(&snapshot::json_bytes(&snapshot).unwrap())).is_err()
    );
}

#[tokio::test]
async fn conditional_rule_maps_to_conditional_ledger_question() {
    let source = r#"
types = { type[tradition] = { path = "game/common/traditions" } }
tradition = { subtype[special] = { unlocks_agenda = scalar } }
"#;
    let ledger = ledger::inventory(&BTreeMap::from([("traditions.cwt".into(), source.into())]));
    let mut snapshot = recorded_snapshot().await;
    qualify_as_live(&mut snapshot);
    let rule = snapshot
        .rules
        .iter_mut()
        .find(|rule| rule.id == "field:common/traditions/unlocks_agenda#existence")
        .unwrap();
    rule.conditions = vec!["subtype[special]".into()];
    let report =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();
    let claim = ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "field_existence"
                && claim.subject == ["tradition", "subtype[special]", "unlocks_agenda"]
        })
        .unwrap();
    assert!(
        report
            .claims
            .iter()
            .find(|item| item.claim == claim.id)
            .unwrap()
            .covered
    );
}

#[tokio::test]
async fn reference_facet_does_not_conflict_with_value_form() {
    let mut snapshot = recorded_snapshot().await;
    qualify_as_live(&mut snapshot);
    let subject = "field:common/traditions/unlocks_agenda";
    snapshot
        .gaps
        .retain(|gap| gap.id != format!("{subject}#reference"));
    let base = snapshot
        .rules
        .iter()
        .find(|rule| rule.id == format!("{subject}#value_form"))
        .unwrap()
        .clone();
    snapshot.rules.push(snapshot::Rule {
        id: format!("{subject}#reference"),
        subject: subject.into(),
        property: "reference".into(),
        conditions: Vec::new(),
        answer: json!({"target":"agenda"}),
        evidence: base.evidence,
    });
    let ledger = ledger();
    let report =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();
    let form = ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "value_form" && claim.subject == ["tradition", "unlocks_agenda"]
        })
        .unwrap();
    assert!(
        report
            .claims
            .iter()
            .find(|item| item.claim == form.id)
            .unwrap()
            .covered
    );
}

#[tokio::test]
async fn comparison_reports_agreement_difference_gaps_and_atlas_only_without_scoring() {
    let source = r#"
types = { type[tradition] = { path = "game/common/traditions" } }
tradition = { unlocks_agenda = int }
"#;
    let ledger = ledger::inventory(&BTreeMap::from([("traditions.cwt".into(), source.into())]));
    let mut rules = recorded_snapshot().await;
    let field = "field:common/traditions/unlocks_agenda";
    rules.gaps.retain(|gap| gap.subject != field);
    let form = rules
        .rules
        .iter_mut()
        .find(|rule| rule.id == format!("{field}#value_form"))
        .unwrap();
    form.answer = json!({"form":"boolean"});
    let bytes = snapshot::json_bytes(&rules).unwrap();
    let comparison = coverage::comparison::evaluate(&ledger, &bytes).unwrap();
    let coverage = coverage::evaluate(&ledger, Some(&bytes)).unwrap();
    assert_eq!(
        coverage.totals.atlas_owned.covered, 0,
        "recorded evidence never earns coverage"
    );
    let claim_status = |property: &str| {
        let claim = ledger
            .claims
            .iter()
            .find(|claim| {
                claim.property == property
                    && claim
                        .subject
                        .last()
                        .is_some_and(|last| last == "unlocks_agenda")
            })
            .unwrap();
        comparison
            .entries
            .iter()
            .find(|entry| entry.claim.as_deref() == Some(&claim.id))
            .unwrap()
    };
    assert_eq!(
        claim_status("field_existence").status,
        coverage::comparison::Status::Same
    );
    let different = claim_status("value_form");
    assert_eq!(different.status, coverage::comparison::Status::Different);
    assert_eq!(different.config_answer.as_deref(), Some("int"));
    assert_eq!(different.atlas_answers, vec![json!({"form":"boolean"})]);
    assert!(
        comparison
            .entries
            .iter()
            .any(|entry| entry.status == coverage::comparison::Status::AtlasOnly)
    );

    rules
        .rules
        .retain(|rule| rule.id != format!("{field}#value_form"));
    rules.gaps.push(snapshot::Gap {
        id: format!("{field}#value_form"),
        subject: field.into(),
        property: "value_form".into(),
        reason: "reader evidence incomplete".into(),
        owner: None,
        native_gaps: Vec::new(),
        evidence: Vec::new(),
    });
    let report =
        coverage::comparison::evaluate(&ledger, &snapshot::json_bytes(&rules).unwrap()).unwrap();
    let claim = ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "value_form"
                && claim
                    .subject
                    .last()
                    .is_some_and(|last| last == "unlocks_agenda")
        })
        .unwrap();
    let entry = report
        .entries
        .iter()
        .find(|entry| entry.claim.as_deref() == Some(&claim.id))
        .unwrap();
    assert_eq!(entry.status, coverage::comparison::Status::MissingFromAtlas);
    assert_eq!(entry.gaps, vec!["reader evidence incomplete"]);
}
