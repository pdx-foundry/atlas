use pdx_atlas::{extraction, snapshot};
use pdx_native::{Basis, Disposal, GameOptions, Native};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

fn recording() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452")
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
    let recorded_registries: Vec<_> = extraction
        .registries
        .as_ref()
        .unwrap()
        .value
        .iter()
        .map(|registry| registry.name.clone())
        .collect();
    assert_eq!(first.coverage.registries, recorded_registries);
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
    assert!(snapshot.rules.iter().any(|rule| rule.id
        == "field:common/traditions/unlocks_agenda#repeat_behavior"
        && rule.answer == "Replace"));
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
    assert!(
        category_storage
            .reason
            .contains("No proven direct storage decoder")
    );
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
    assert!(
        !snapshot
            .rules
            .iter()
            .any(|rule| rule.id == format!("{id}#repeat_behavior"))
    );
    assert!(
        snapshot
            .gaps
            .iter()
            .any(|gap| gap.id == format!("{id}#occurrences.maximum"))
    );
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

#[tokio::test]
async fn unresolved_value_forms_keep_only_their_field_gaps_and_correct_reason() {
    let snapshot = snapshot::assemble(&recorded().await).unwrap();
    let ring = snapshot
        .gaps
        .iter()
        .find(|gap| gap.id == "field:map/galaxy/ring#value_form.unresolved")
        .unwrap();
    assert_eq!(ring.native_gaps, ring.evidence[0].native_gaps);
    assert!(ring.native_gaps.iter().all(|gap| gap.subject
        == Some(pdx_native::GapSubject::Field {
            name: "ring".into()
        })));
    let unresolved = snapshot
        .gaps
        .iter()
        .find(|gap| gap.id == "field:interface/resource_groups/localization#value_form.unresolved")
        .unwrap();
    assert_eq!(
        unresolved.reason,
        "Native did not establish the unconditional reader's value form"
    );
    assert_eq!(unresolved.owner, None);
}

#[tokio::test]
async fn gap_names_attach_to_one_field_path() {
    use pdx_native::{Field, FieldMembers, Gap, GapKind, GapSubject};
    let mut extraction = recorded().await;
    let traditions = extraction.fields[extraction::TRADITIONS]
        .as_ref()
        .unwrap()
        .value
        .clone();
    let template = |name: &str| {
        traditions
            .iter()
            .find(|field| field.name == name)
            .unwrap()
            .clone()
    };
    let mut child = template("unlocks_agenda");
    child.name = "child".into();
    let parent = |name: &str| Field {
        name: name.into(),
        members: FieldMembers::Fields(vec![child.clone()]),
        ..template("on_enabled")
    };
    let gap = |subject| Gap {
        kind: GapKind::UnresolvedStorage,
        subject: Some(subject),
        detail: "test storage boundary".into(),
    };
    let leaf = gap(GapSubject::Field {
        name: "child".into(),
    });
    let dotted = gap(GapSubject::Field {
        name: "parent.child".into(),
    });
    let key_path = gap(GapSubject::KeyPath {
        path: vec!["parent".into(), "child".into(), "factor".into()],
    });
    let mut assemble = |fields: Vec<Field>, gaps: Vec<Gap>| {
        let answer = extraction
            .fields
            .get_mut(extraction::TRADITIONS)
            .unwrap()
            .as_mut()
            .unwrap();
        answer.value = fields;
        answer.gaps = gaps;
        snapshot::assemble(&extraction).unwrap()
    };
    let read_gaps = |snapshot: &snapshot::Snapshot, path: &str| {
        snapshot
            .rules
            .iter()
            .find(|rule| rule.id == format!("field:common/traditions/{path}#read"))
            .unwrap()
            .evidence[0]
            .native_gaps
            .clone()
    };
    let unjoined = |snapshot: &snapshot::Snapshot| {
        snapshot
            .gaps
            .iter()
            .find(|gap| gap.id == "registry:common/traditions#field_gap_subjects")
            .cloned()
    };

    // A unique nested leaf, a dotted path and a key path below the field's members all join it.
    let nested = assemble(
        vec![parent("parent")],
        vec![leaf.clone(), dotted.clone(), key_path.clone()],
    );
    assert_eq!(
        read_gaps(&nested, "parent/child"),
        [leaf.clone(), dotted, key_path]
    );
    assert!(unjoined(&nested).is_none());

    // One name is the root field of that name.
    let root = assemble(vec![parent("parent"), child.clone()], vec![leaf.clone()]);
    assert_eq!(read_gaps(&root, "child"), std::slice::from_ref(&leaf));
    assert!(read_gaps(&root, "parent/child").is_empty());

    // Without a root field, a name that two nested paths end with joins neither.
    let ambiguous = assemble(vec![parent("parent"), parent("other")], vec![leaf.clone()]);
    for path in ["parent/child", "other/child"] {
        assert!(read_gaps(&ambiguous, path).is_empty());
    }
    let gap = unjoined(&ambiguous).unwrap();
    assert_eq!(gap.native_gaps, [leaf]);
    assert!(gap.reason.contains("parent/child") && gap.reason.contains("other/child"));
}

#[tokio::test]
async fn version_two_still_accepts_legacy_ticket_owners() {
    let mut snapshot = snapshot::assemble(&recorded().await).unwrap();
    snapshot.gaps[0].owner = Some("SDK-548".into());
    snapshot::verify(&snapshot).unwrap();
    let schema: Value = serde_json::from_str(include_str!(
        "../docs/contract/rule-snapshot-v2.schema.json"
    ))
    .unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&json!(snapshot))
        .unwrap();
    snapshot.gaps[0].owner = Some("SDK-invalid".into());
    assert!(snapshot::verify(&snapshot).is_err());
}

#[tokio::test]
async fn repeat_facts_require_an_unconditional_successful_read() {
    use pdx_native::{FieldCondition, FieldReadAlternative, FieldReadOutcome, RepeatBehavior};
    let mut extraction = recorded().await;
    let fields = extraction
        .fields
        .get_mut(extraction::TRADITIONS)
        .unwrap()
        .as_mut()
        .unwrap();
    let base = fields
        .value
        .iter()
        .find(|field| field.name == "unlocks_agenda")
        .unwrap()
        .clone();
    for (name, condition, repeat) in [
        ("replace", FieldCondition::Always, RepeatBehavior::Replace),
        (
            "accumulate",
            FieldCondition::Always,
            RepeatBehavior::Accumulate,
        ),
        ("merges", FieldCondition::Always, RepeatBehavior::Merges),
        ("unknown", FieldCondition::Always, RepeatBehavior::Unknown),
        (
            "conditional",
            FieldCondition::Unresolved,
            RepeatBehavior::Replace,
        ),
        (
            "unresolved",
            FieldCondition::Always,
            RepeatBehavior::Replace,
        ),
        ("rejected", FieldCondition::Always, RepeatBehavior::Replace),
    ] {
        let mut field = base.clone();
        field.name = name.into();
        let mut shape = field.shape;
        shape.repeat = repeat;
        field.read = vec![FieldReadAlternative {
            condition,
            outcome: match name {
                "unresolved" => FieldReadOutcome::Unresolved,
                "rejected" => FieldReadOutcome::Rejected,
                _ => FieldReadOutcome::Read {
                    reader: field.reader.clone(),
                    shape,
                },
            },
        }];
        // The summary cannot override the read alternative's actual result.
        field.shape.repeat = RepeatBehavior::Replace;
        fields.value.push(field);
    }
    let snapshot = snapshot::assemble(&extraction).unwrap();
    for (name, expected) in [
        ("replace", "Replace"),
        ("accumulate", "Accumulate"),
        ("merges", "Merges"),
    ] {
        let subject = format!("field:common/traditions/{name}");
        let rule = snapshot
            .rules
            .iter()
            .find(|rule| rule.id == format!("{subject}#repeat_behavior"))
            .unwrap();
        assert_eq!(rule.answer, expected);
        assert!(
            !snapshot
                .gaps
                .iter()
                .any(|gap| gap.id == format!("{subject}#occurrences.maximum"))
        );
        assert!(
            snapshot
                .gaps
                .iter()
                .any(|gap| gap.id == format!("{subject}#occurrences.minimum"))
        );
    }
    for name in ["unknown", "conditional", "unresolved", "rejected"] {
        let subject = format!("field:common/traditions/{name}");
        assert!(
            !snapshot
                .rules
                .iter()
                .any(|rule| rule.id == format!("{subject}#repeat_behavior"))
        );
        assert!(
            snapshot
                .gaps
                .iter()
                .any(|gap| gap.id == format!("{subject}#occurrences.maximum"))
        );
    }
    assert!(
        snapshot
            .rules
            .iter()
            .all(|rule| rule.property != "occurrences.maximum")
    );
}

#[tokio::test]
async fn council_agenda_fields_carry_block_scope_value_and_reference_answers() {
    let snapshot = snapshot::assemble(&recorded().await).unwrap();
    let field = |name: &str| format!("field:common/council_agendas/{name}");
    let rule = |id: String| snapshot.rules.iter().find(|rule| rule.id == id);
    let gap = |id: String| snapshot.gaps.iter().find(|gap| gap.id == id);

    let potential = field("potential");
    assert!(
        rule(format!("{potential}#block_family"))
            .unwrap()
            .answer
            .to_string()
            .contains("Trigger")
    );
    assert!(gap(format!("{potential}#nested_grammar")).is_none());
    for name in ["potential", "effect"] {
        let this = &rule(format!("{}#read_scope", field(name))).unwrap().answer;
        assert_eq!(
            rule(format!("{}#scope_context", field(name)))
                .unwrap()
                .answer["this"],
            *this
        );
    }

    let ai_weight = field("ai_weight");
    assert!(gap(format!("{ai_weight}#nested_grammar")).is_none());
    assert!(rule(format!("{ai_weight}#read_scope")).is_some());
    assert!(
        gap(format!("{ai_weight}#scope_context"))
            .unwrap()
            .reason
            .contains("not every evaluation context")
    );

    assert_eq!(
        rule(format!("{}#value_form", field("agenda_cost")))
            .unwrap()
            .answer["form"],
        "scoped_number"
    );
    assert!(rule(format!("{}#reference", field("finish_modifier"))).is_some());
    assert!(gap(format!("{}#reference", field("finish_modifier"))).is_none());

    let modifier = field("modifier");
    assert!(rule(format!("{modifier}#accepted_categories")).is_some());
    assert!(rule(format!("{modifier}#scope_context")).is_none());
    assert!(gap(format!("{modifier}#scope_context")).is_none());
}

#[tokio::test]
async fn unestablished_block_parts_keep_their_gaps() {
    use pdx_native::{BlockFamily, FieldMembers};
    let mut extraction = recorded().await;
    let effect = extraction
        .fields
        .get_mut("common/council_agendas")
        .unwrap()
        .as_mut()
        .unwrap()
        .value
        .iter_mut()
        .find(|field| field.name == "effect")
        .unwrap();
    effect.reader.family = BlockFamily::Unknown;
    effect.members = FieldMembers::Unresolved;
    effect.entry_contexts.clear();

    let snapshot = snapshot::assemble(&extraction).unwrap();
    let id = "field:common/council_agendas/effect";
    let gap = |property: &str| {
        snapshot
            .gaps
            .iter()
            .find(|gap| gap.id == format!("{id}#{property}"))
    };

    assert!(gap("nested_grammar").is_some());
    assert!(
        gap("scope_context")
            .unwrap()
            .reason
            .contains("not every evaluation context")
    );
    assert!(
        snapshot
            .rules
            .iter()
            .any(|rule| rule.id == format!("{id}#read_scope"))
    );
}

#[tokio::test]
async fn failed_grammar_and_derived_name_answers_are_gaps_and_fail_extraction() {
    use pdx_native::{Error, Operation};
    let mut extraction = recorded().await;
    assert!(extraction.complete());
    let failure = |operation| Error::Observation {
        operation,
        reason: "reader stopped".into(),
    };
    extraction.language.effect_grammars.insert(
        "add_building".into(),
        Err(failure(Operation::CommandGrammar)),
    );
    assert!(!extraction.complete());
    extraction.language.effect_grammars = recorded().await.language.effect_grammars;
    extraction.derived_names.insert(
        "common/council_agendas".into(),
        Err(failure(Operation::DerivedNames)),
    );
    assert!(!extraction.complete());
    extraction.language.effect_grammars.insert(
        "add_building".into(),
        Err(failure(Operation::CommandGrammar)),
    );

    let snapshot = snapshot::assemble(&extraction).unwrap();
    for id in [
        "effect:add_building#arguments",
        "effect:add_building#forms",
        "effect:add_building#scope_context",
        "registry:common/council_agendas#derived_names",
    ] {
        assert!(
            snapshot
                .gaps
                .iter()
                .any(|gap| gap.id == id && gap.reason.contains("reader stopped")),
            "{id}"
        );
    }
}
