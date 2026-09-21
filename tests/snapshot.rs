use pdx_atlas::{extraction, snapshot};
use pdx_native::{Basis, Disposal, Field, GameOptions, Native};
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
        "../docs/contract/rule-snapshot-v1.schema.json"
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
    assert!(first.snapshot.name.starts_with("stellaris-registry-rules/"));
    for subject in first
        .subjects
        .iter()
        .filter(|subject| subject.kind == "field")
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
            assert!(first.sources.contains_key(&evidence.source));
            completeness.insert(format!("{:?}", evidence.completeness));
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
    let category_validation = snapshot
        .gaps
        .iter()
        .find(|gap| gap.id == "field:common/tradition_categories/desc#validation")
        .unwrap();
    assert!(
        category_validation
            .reason
            .contains("outside this registry binding")
    );
    assert!(
        snapshot
            .rules
            .iter()
            .all(|rule| rule.property != "optional")
    );
}

#[tokio::test]
async fn unknown_and_conditional_readers_leave_named_gaps() {
    let mut extraction = recorded().await;
    let fields = extraction
        .fields
        .get_mut(extraction::TRADITIONS)
        .unwrap()
        .as_mut()
        .unwrap();
    let unknown: Field = serde_json::from_value(
        json!({"name":"new_unknown","reader":{"id":null,"kind":"Unknown"},"conditional":false}),
    )
    .unwrap();
    let conditional: Field = serde_json::from_value(json!({"name":"new_conditional","reader":{"id":"reader-test","kind":"String"},"conditional":true})).unwrap();
    fields.value.extend([unknown, conditional]);
    let snapshot = snapshot::assemble(&extraction).unwrap();
    for (field, property) in [
        ("new_unknown", "value_form"),
        ("new_conditional", "conditions"),
        ("new_conditional", "value_form"),
    ] {
        let id = format!("field:common/traditions/{field}#{property}");
        let gap = snapshot.gaps.iter().find(|gap| gap.id == id).unwrap();
        assert_eq!(gap.owner, None);
        assert!(!gap.reason.is_empty());
    }
    assert!(
        !snapshot
            .rules
            .iter()
            .any(|rule| rule.id == "field:common/traditions/new_conditional#value_form")
    );
}
