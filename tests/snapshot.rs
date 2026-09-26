use pdx_atlas::{extraction, snapshot};
use pdx_native::{Basis, Disposal, GameOptions, Native};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

fn recording() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m45")
}

async fn recorded() -> extraction::Extraction {
    let native = Native::from_recorded_answers(recording()).unwrap();
    extraction::collect(&native, || GameOptions::new(Command::new("unused"))).await
}

#[tokio::test]
async fn recorded_snapshot_validates_and_is_byte_stable() {
    let extraction = recorded().await;
    let first = snapshot::assemble(&extraction).unwrap();
    snapshot::verify(&first).unwrap();
    let schema: Value = serde_json::from_str(include_str!(
        "../docs/contract/rule-snapshot-v2.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let value = serde_json::to_value(&first).unwrap();
    let errors: Vec<_> = validator
        .iter_errors(&value)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        snapshot::json_bytes(&first).unwrap(),
        snapshot::json_bytes(&snapshot::assemble(&extraction).unwrap()).unwrap()
    );
    assert!(
        first
            .sources
            .values()
            .all(|source| source.basis == Basis::Recorded)
    );
    assert_eq!(first.coverage.whole_registry_validity, "not_established");
    assert!(first.snapshot.name.starts_with("stellaris-rules/"));
    for subject in first
        .subjects
        .iter()
        .filter(|subject| subject.kind == snapshot::SubjectKind::Field)
    {
        let id = format!("{}#value_form", subject.id);
        assert!(
            first.rules.iter().any(|rule| rule.id == id)
                || first.gaps.iter().any(|gap| gap.id == id),
            "{id}"
        );
    }
    let mut completeness = std::collections::BTreeSet::new();
    for rule in &first.rules {
        assert!(!rule.evidence.is_empty());
        for evidence in &rule.evidence {
            let answer = &first.answers[&evidence.answer];
            assert!(first.sources.contains_key(&answer.source));
            completeness.insert(format!("{:?}", answer.completeness));
        }
    }
    assert_eq!(
        completeness,
        ["Complete".to_owned(), "Partial".to_owned()].into()
    );
    let string_schemas: std::collections::BTreeSet<_> = first
        .rules
        .iter()
        .filter_map(|rule| rule.answer.get("schema").and_then(Value::as_str))
        .collect();
    assert!(string_schemas.len() > 1);
    assert_eq!(first.schemas.definitions.len(), string_schemas.len());
    assert_eq!(first.coverage.registries.len(), 164);
    assert!(
        first
            .rules
            .iter()
            .flat_map(|rule| &rule.evidence)
            .any(|evidence| !evidence.native_gaps.is_empty())
    );
}

#[tokio::test]
async fn incomplete_or_undisposed_fixtures_cannot_reuse_complete_rules() {
    let complete = recorded().await;
    let complete_snapshot = snapshot::assemble(&complete).unwrap();

    let mut incomplete = recorded().await;
    incomplete.sessions[0].disposal = Ok(Disposal::Unconfirmed("test".into()));
    let failed_snapshot = snapshot::assemble(&incomplete).unwrap();
    assert_ne!(
        complete_snapshot.snapshot.name,
        failed_snapshot.snapshot.name
    );
    assert!(
        failed_snapshot
            .gaps
            .iter()
            .any(|gap| gap.property == "fixture.tradition_outcomes")
    );
    assert!(
        !failed_snapshot
            .rules
            .iter()
            .any(|rule| rule.property == "occurrences.parser_accepted"
                && rule.subject.starts_with("field:common/traditions/"))
    );
}

#[tokio::test]
async fn fixture_counts_and_gaps_are_bounded() {
    let snapshot = snapshot::assemble(&recorded().await).unwrap();
    let rule = snapshot
        .rules
        .iter()
        .find(|rule| {
            rule.id == "field:common/traditions/unlocks_agenda#occurrences.parser_accepted"
        })
        .unwrap();
    assert_eq!(rule.answer["counts"], json!([0, 1, 2]));
    assert!(
        rule.answer["rejected_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value["text"] == "Malformed token")
    );
    assert!(
        rule.evidence
            .iter()
            .any(|evidence| evidence.location.contains("atlas_malformed"))
    );
    assert!(
        snapshot
            .gaps
            .iter()
            .any(|gap| gap.id == "field:common/traditions/unlocks_agenda#occurrences.minimum")
    );
    assert!(
        snapshot
            .gaps
            .iter()
            .any(|gap| gap.id == "field:common/traditions/unlocks_agenda#occurrences.maximum")
    );
    // Category diagnostics are complete, so the first missing dimension is storage.
    assert!(
        snapshot
            .gaps
            .iter()
            .all(|gap| gap.id != "field:common/tradition_categories/desc#validation")
    );
    let category_storage = snapshot
        .gaps
        .iter()
        .find(|gap| gap.id == "field:common/tradition_categories/desc#storage")
        .unwrap();
    assert!(category_storage.reason.contains("no storage decoder"));
    assert!(
        snapshot
            .rules
            .iter()
            .all(|rule| rule.property != "optional")
    );
}

#[tokio::test]
async fn conditional_branches_and_unknown_siblings_survive_all_consumers() {
    use pdx_atlas::{coverage, ledger};
    use pdx_native::{FieldCondition, FieldReadAlternative, FieldReadOutcome};
    use std::collections::BTreeMap;
    let mut extraction = recorded().await;
    let fields = extraction
        .fields
        .get_mut(extraction::TRADITIONS)
        .unwrap()
        .as_mut()
        .unwrap();
    let mut field = fields
        .value
        .iter()
        .find(|field| field.name == "unlocks_agenda")
        .unwrap()
        .clone();
    field.name = "conditional".into();
    let outcome = FieldReadOutcome::Read {
        reader: field.reader.clone(),
        shape: field.shape,
    };
    field.read = vec![
        FieldReadAlternative {
            condition: FieldCondition::FieldZero {
                path: vec!["flag".into()],
                zero: true,
            },
            outcome: outcome.clone(),
        },
        FieldReadAlternative {
            condition: FieldCondition::FieldZero {
                path: vec!["flag".into()],
                zero: false,
            },
            outcome,
        },
        FieldReadAlternative {
            condition: FieldCondition::Unresolved,
            outcome: FieldReadOutcome::Unresolved,
        },
    ];
    let alternatives = json!(field.read);
    fields.value.push(field.clone());
    let mut snapshot = snapshot::assemble(&extraction).unwrap();
    let id = "field:common/traditions/conditional";
    assert_eq!(
        snapshot
            .rules
            .iter()
            .find(|r| r.id == format!("{id}#read"))
            .unwrap()
            .answer,
        alternatives
    );
    assert!(
        snapshot
            .gaps
            .iter()
            .any(|g| g.id == format!("{id}#value_form.unresolved"))
    );
    let ledger = ledger::inventory(&BTreeMap::from([("test.cwt".into(), "types = { type[tradition] = { path = \"game/common/traditions\" } }\ntradition = { conditional = scalar }".into())]));
    let comparison = coverage::comparison::evaluate(
        &ledger,
        &snapshot::json_bytes(&snapshot).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let entry = comparison
        .entries
        .iter()
        .find(|e| {
            e.property.as_deref() == Some("value_form")
                && e.config_answer.as_deref() == Some("scalar")
        })
        .unwrap();
    assert_eq!(entry.status, coverage::comparison::Status::MissingFromAtlas);

    assert_eq!(entry.atlas_answers[0]["alternatives"], alternatives);
    assert!(!entry.gaps.is_empty());
    let keys: BTreeMap<_, _> = snapshot
        .sources
        .iter()
        .map(|(key, source)| (key.clone(), format!("{}@static_analysis", source.method)))
        .collect();
    snapshot.sources = snapshot
        .sources
        .into_iter()
        .map(|(key, mut source)| {
            source.basis = Basis::StaticAnalysis;
            (keys[&key].clone(), source)
        })
        .collect();
    for answer in snapshot.answers.values_mut() {
        answer.source = keys[&answer.source].clone();
    }
    let coverage =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();
    let question = ledger
        .claims
        .iter()
        .find(|c| c.property == "value_form" && c.subject == ["tradition", "conditional"])
        .unwrap();
    assert!(
        !coverage
            .claims
            .iter()
            .find(|c| c.claim == question.id)
            .unwrap()
            .covered
    );
}

#[tokio::test]
async fn duplicate_rule_and_field_identities_are_invalid() {
    let mut extraction = recorded().await;
    let mut snapshot = snapshot::assemble(&extraction).unwrap();
    snapshot.rules.push(snapshot.rules[0].clone());
    assert!(snapshot::verify(&snapshot).is_err());
    let fields = &mut extraction
        .fields
        .get_mut(extraction::TRADITIONS)
        .unwrap()
        .as_mut()
        .unwrap()
        .value;
    fields.push(fields[0].clone());
    assert!(snapshot::assemble(&extraction).is_err());
}

#[tokio::test]
async fn block_shape_does_not_require_a_reader_identity() {
    use pdx_native::{FieldCondition, FieldReadAlternative, FieldReadOutcome, ReaderKind};
    let mut extraction = recorded().await;
    let fields = extraction
        .fields
        .get_mut(extraction::TRADITIONS)
        .unwrap()
        .as_mut()
        .unwrap();
    let field = fields
        .value
        .iter_mut()
        .find(|field| field.name == "on_enabled")
        .unwrap();
    field.reader.id = None;
    field.reader.kind = ReaderKind::Block;
    field.read = vec![FieldReadAlternative {
        condition: FieldCondition::Always,
        outcome: FieldReadOutcome::Read {
            reader: field.reader.clone(),
            shape: field.shape,
        },
    }];
    let snapshot = snapshot::assemble(&extraction).unwrap();
    let rule = snapshot
        .rules
        .iter()
        .find(|r| r.id == "field:common/traditions/on_enabled#value_form")
        .unwrap();
    assert_eq!(rule.answer["form"], "block");
    assert!(rule.answer["reader"].is_null());
}

#[tokio::test]
async fn published_failure_reasons_keep_details_without_debug_syntax() {
    use pdx_native::{Error, Operation};
    let mut extraction = recorded().await;
    extraction.fields.insert(
        "common/traditions".into(),
        Err(Error::Observation {
            operation: Operation::RegistryFields,
            reason: "reader stopped".into(),
        }),
    );
    extraction.loaded_modifiers.disposal = Ok(Disposal::Unconfirmed("worker lost".into()));
    let snapshot = snapshot::assemble(&extraction).unwrap();
    let reason = &snapshot
        .gaps
        .iter()
        .find(|g| g.id == "registry:common/traditions#fields")
        .unwrap()
        .reason;
    assert_eq!(
        reason,
        "Native field question failed: Observation not established: reader stopped"
    );
    assert!(snapshot.gaps.iter().any(|g| g.reason == "Native loaded-modifier session disposal failed: process disposal unconfirmed: worker lost"));
    assert_eq!(
        snapshot.snapshot.name,
        snapshot::assemble(&extraction).unwrap().snapshot.name
    );
}

#[tokio::test]
async fn nested_paths_and_use_conditions_keep_their_stage_and_parent_limit() {
    use pdx_native::{FieldCondition, FieldMembers, FieldReadAlternative, FieldReadOutcome};
    let mut extraction = recorded().await;
    let fields = extraction
        .fields
        .get_mut(extraction::TRADITIONS)
        .unwrap()
        .as_mut()
        .unwrap();
    let mut child = fields
        .value
        .iter()
        .find(|field| field.name == "unlocks_agenda")
        .unwrap()
        .clone();
    child.name = "child".into();
    child.uses = serde_json::from_value(json!([{"id":"opaque-use","condition":{"All":["Unresolved",{"FieldZero":{"path":["parent","flag"],"zero":true}}]}}])).unwrap();
    let uses = json!(child.uses);
    let mut parent = fields
        .value
        .iter()
        .find(|field| field.name == "on_enabled")
        .unwrap()
        .clone();
    parent.name = "parent".into();
    parent.read = vec![FieldReadAlternative {
        condition: FieldCondition::Unresolved,
        outcome: FieldReadOutcome::Read {
            reader: parent.reader.clone(),
            shape: parent.shape,
        },
    }];
    parent.members = FieldMembers::Fields(vec![child]);
    fields.value.push(parent);
    let snapshot = snapshot::assemble(&extraction).unwrap();
    let id = "field:common/traditions/parent/child";
    let subject = snapshot.subjects.iter().find(|s| s.id == id).unwrap();
    assert_eq!(subject.registry.as_deref(), Some("common/traditions"));
    assert_eq!(subject.field.as_deref(), Some("parent/child"));
    assert_eq!(subject.conditional, Some(true));
    let rule = snapshot
        .rules
        .iter()
        .find(|r| r.id == format!("{id}#uses"))
        .unwrap();
    assert_eq!(rule.answer["stage"], "stored_value_selection");
    assert_eq!(rule.answer["selections"], uses);
    assert!(
        snapshot
            .gaps
            .iter()
            .any(|g| g.id == format!("{id}#value_form.unresolved"))
    );
}
