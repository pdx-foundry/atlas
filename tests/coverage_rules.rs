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

/// Coverage and comparison gaps for the claims of config `sources`, against the recorded M452
/// answers qualified as live and changed by `edit`.
struct SharedDirectory {
    ledger: ledger::Ledger,
    coverage: coverage::Report,
    comparison: coverage::comparison::Report,
}

impl SharedDirectory {
    /// Evaluates `sources` after `edit` changes the recorded snapshot; the edited answers are then
    /// qualified as live.
    async fn evaluate(
        sources: &[(&str, &str)],
        edit: impl FnOnce(&mut snapshot::Snapshot),
    ) -> Self {
        let ledger = ledger::inventory(
            &sources
                .iter()
                .map(|(file, source)| ((*file).into(), (*source).into()))
                .collect(),
        );
        let mut snapshot = recorded_snapshot().await;
        edit(&mut snapshot);
        qualify_as_live(&mut snapshot);
        let bytes = snapshot::json_bytes(&snapshot).unwrap();

        Self {
            coverage: coverage::evaluate(&ledger, Some(&bytes)).unwrap(),
            comparison: coverage::comparison::evaluate(&ledger, &bytes, &Default::default())
                .unwrap(),
            ledger,
        }
    }

    /// The ID of the claim at `subject` with `property`; panics when there is none.
    fn claim_id(&self, property: &str, subject: &[&str]) -> &str {
        &self
            .ledger
            .claims
            .iter()
            .find(|claim| claim.property == property && claim.subject == subject)
            .unwrap_or_else(|| panic!("no {property} claim at {subject:?}"))
            .id
    }

    /// Whether the claim at `subject` with `property` is covered.
    fn covered(&self, property: &str, subject: &[&str]) -> bool {
        let id = self.claim_id(property, subject);

        self.coverage
            .claims
            .iter()
            .find(|assessment| assessment.claim == id)
            .unwrap()
            .covered
    }

    /// The comparison gaps of the claim at `subject` with `property`.
    fn gaps(&self, property: &str, subject: &[&str]) -> &[String] {
        let id = self.claim_id(property, subject);

        &self
            .comparison
            .entries
            .iter()
            .find(|entry| entry.claim.as_deref() == Some(id))
            .unwrap()
            .gaps
    }
}

/// A `tradition` directory with a second type written by `selector_lines`.
async fn swapped_tradition(selector_lines: &str) -> SharedDirectory {
    let source = format!(
        r#"
types = {{
    type[tradition] = {{ path = "game/common/traditions" }}
    {selector_lines}
}}
tradition = {{ unlocks_agenda = scalar }}
"#
    );

    SharedDirectory::evaluate(&[("traditions.cwt", &source)], |_| {}).await
}

#[tokio::test]
async fn root_and_swapped_types_join_through_their_selectors() {
    let swap = r#"
    ## type_key_filter = tradition_swap
    type[swapped_tradition] = {
        base_type = tradition
        path = "game/common/traditions"
        skip_root_key = any
        name_field = name
    }"#;
    let directory = swapped_tradition(swap).await;

    for (property, subject) in [
        ("type_existence", &["types", "type[tradition]"][..]),
        ("loader_path", &["types", "type[tradition]", "path"]),
        ("field_existence", &["tradition", "unlocks_agenda"]),
        ("type_existence", &["types", "type[swapped_tradition]"]),
        ("loader_path", &["types", "type[swapped_tradition]", "path"]),
    ] {
        assert!(directory.covered(property, subject), "{subject:?}");
    }
}

#[tokio::test]
async fn nested_type_joins_through_a_skipped_key_path() {
    let swap = r#"
    ## type_key_filter = name
    type[swapped_tradition] = {
        base_type = tradition
        path = "game/common/traditions"
        skip_root_key = { any tradition_swap }
    }"#;
    let directory = swapped_tradition(swap).await;

    assert!(directory.covered("type_existence", &["types", "type[swapped_tradition]"]));
}

#[tokio::test]
async fn mismatched_nested_selector_keeps_a_gap_that_names_it() {
    for (swap, selector) in [
        (
            r#"
    ## type_key_filter = other_swap
    type[swapped_tradition] = {
        base_type = tradition
        path = "game/common/traditions"
        skip_root_key = any
    }"#,
            "`type_key_filter = other_swap`, `skip_root_key = any`, `base_type = tradition`",
        ),
        (
            r#"
    ## type_key_filter = tradition_swap
    type[swapped_tradition] = {
        base_type = other
        path = "game/common/traditions"
        skip_root_key = any
    }"#,
            "`base_type = other`",
        ),
        (
            r#"
    ## type_key_filter = swap_type
    type[swapped_tradition] = {
        base_type = tradition
        path = "game/common/traditions"
        skip_root_key = { any swappable_data }
    }"#,
            "`skip_root_key = { any swappable_data }`",
        ),
    ] {
        let directory = swapped_tradition(swap).await;

        for (property, subject) in [
            ("type_existence", &["types", "type[swapped_tradition]"][..]),
            ("loader_path", &["types", "type[swapped_tradition]", "path"]),
        ] {
            assert!(!directory.covered(property, subject), "{swap}");
            assert!(
                directory
                    .gaps(property, subject)
                    .iter()
                    .any(|gap| gap.contains(selector)),
                "{swap}"
            );
        }

        assert!(directory.covered("field_existence", &["tradition", "unlocks_agenda"]));
    }
}

#[tokio::test]
async fn event_namespace_selector_stays_unresolved_and_takes_no_event_kind_claim() {
    let events = r#"
types = {
    type[event] = {
        path = "game/events"
        ## type_key_filter = country_event
        subtype[country] = {}
    }
}
event = { subtype[country] = { trigger = scalar } }
"#;
    let namespaces = r#"
types = {
    ## type_key_filter = namespace
    type[event_namespace] = {
        path = "game/events"
        name_field = "-"
    }
}
event_namespace = scalar
"#;
    let directory = SharedDirectory::evaluate(
        &[
            ("events/events.cwt", events),
            ("events/event_namespaces.cwt", namespaces),
        ],
        |snapshot| {
            let template = snapshot
                .rules
                .iter()
                .find(|rule| rule.id == "registry:common/traditions#existence")
                .unwrap()
                .clone();

            snapshot.subjects.push(snapshot::Subject {
                id: "registry:events".into(),
                kind: snapshot::SubjectKind::Registry,
                registry: Some("events".into()),
                field: None,
                conditional: None,
                name: None,
            });
            snapshot.subjects.push(snapshot::Subject {
                id: "field:events/trigger".into(),
                kind: snapshot::SubjectKind::Field,
                registry: Some("events".into()),
                field: Some("trigger".into()),
                conditional: Some(false),
                name: None,
            });
            for (subject, conditions) in [
                ("registry:events", vec![]),
                (
                    "field:events/trigger",
                    vec!["root_key[country_event]".into()],
                ),
            ] {
                snapshot.rules.push(snapshot::Rule {
                    id: format!("{subject}#existence"),
                    subject: subject.into(),
                    conditions,
                    ..template.clone()
                });
            }
        },
    )
    .await;

    assert!(directory.covered("type_existence", &["types", "type[event]"]));
    assert!(directory.covered("field_existence", &["event", "subtype[country]", "trigger"]));
    assert!(!directory.covered("type_existence", &["types", "type[event_namespace]"]));
    assert!(
        directory
            .gaps("type_existence", &["types", "type[event_namespace]"])
            .iter()
            .any(|gap| gap.contains("`type_key_filter = namespace`"))
    );
    assert!(!directory.covered("value_form", &["event_namespace"]));
}

/// Coverage of the `species_name` and `species_named_list` types when the first is selected by
/// `type_key_filter {species_filter}`.
async fn species_names(species_filter: &str) -> SharedDirectory {
    let source = format!(
        r#"
types = {{
    ## type_key_filter {species_filter}
    type[species_name] = {{
        path = "game/common/traditions"
        name_field = "name"
    }}
    type[species_named_list] = {{
        path = "game/common/traditions"
        skip_root_key = named_lists
    }}
}}
"#
    );

    SharedDirectory::evaluate(&[("names.cwt", &source)], |_| {}).await
}

#[tokio::test]
async fn excluded_root_key_leaves_the_root_type_and_a_wrapped_type_keeps_its_gap() {
    let directory = species_names("<> named_lists").await;
    let named_list = ["types", "type[species_named_list]"];

    assert!(directory.covered("type_existence", &["types", "type[species_name]"]));
    assert!(!directory.covered("type_existence", &named_list));
    assert!(
        directory
            .gaps("type_existence", &named_list)
            .iter()
            .any(|gap| gap.contains("`skip_root_key = named_lists`"))
    );

    let selected = species_names("= named_lists").await;
    assert!(!selected.covered("type_existence", &["types", "type[species_name]"]));
}

async fn recorded_snapshot() -> snapshot::Snapshot {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452");
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
        let new = format!("{}@live_observation", source.method);
        keys.insert(old, new.clone());
        snapshot.sources.insert(new, source);
    }
    for answer in snapshot.answers.values_mut() {
        answer.source = keys[&answer.source].clone();
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
    for version in [1, 3] {
        snapshot.contract_version = version;
        assert!(
            coverage::evaluate(&ledger(), Some(&snapshot::json_bytes(&snapshot).unwrap())).is_err()
        );
    }
    snapshot.contract_version = snapshot::CONTRACT_VERSION;
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

/// Whether the `unlocks_agenda` existence claim under `subtypes` is covered when Native answers the
/// field under `conditions`.
async fn subtype_coverage(conditions: &[&str], subtypes: &[&str]) -> bool {
    let source = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
        ## type_key_filter = special_tradition
        subtype[special] = {}
        ## type_key_filter = special_tradition
        subtype[gated] = { gated = yes }
        ##type_key_filter=compact_tradition
        subtype[compact] = {}
    }
}
tradition = {
    subtype[special] = {
        unlocks_agenda = scalar
        subtype[compact] = { unlocks_agenda = scalar }
    }
    subtype[gated] = { unlocks_agenda = scalar }
    subtype[compact] = { unlocks_agenda = scalar }
}
"#;
    let ledger = ledger::inventory(&BTreeMap::from([("traditions.cwt".into(), source.into())]));
    let mut snapshot = recorded_snapshot().await;
    qualify_as_live(&mut snapshot);
    let rule = snapshot
        .rules
        .iter_mut()
        .find(|rule| rule.id == "field:common/traditions/unlocks_agenda#existence")
        .unwrap();
    rule.conditions = conditions.iter().map(|&c| c.into()).collect();
    let report =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();
    let mut subject = vec!["tradition"];
    subject.extend(subtypes);
    subject.push("unlocks_agenda");
    let claim = ledger
        .claims
        .iter()
        .find(|claim| claim.property == "field_existence" && claim.subject == subject)
        .unwrap();
    report
        .claims
        .iter()
        .find(|item| item.claim == claim.id)
        .unwrap()
        .covered
}

#[tokio::test]
async fn root_key_credits_the_subtype_that_the_key_alone_selects() {
    assert!(subtype_coverage(&["root_key[special_tradition]"], &["subtype[special]"]).await);
    assert!(subtype_coverage(&["root_key[compact_tradition]"], &["subtype[compact]"]).await);
    assert!(!subtype_coverage(&["root_key[special_tradition]"], &["subtype[gated]"]).await);
}

#[tokio::test]
async fn mismatched_or_mixed_root_key_credits_no_subtype() {
    assert!(!subtype_coverage(&["root_key[other_tradition]"], &["subtype[special]"]).await);
    assert!(
        !subtype_coverage(
            &["root_key[special_tradition]", "subtype[compact]"],
            &["subtype[special]", "subtype[compact]"],
        )
        .await
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
    let comparison = coverage::comparison::evaluate(&ledger, &bytes, &Default::default()).unwrap();
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
            .find(|entry| entry.claim.as_deref() == Some(claim.id.as_str()))
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
    let report = coverage::comparison::evaluate(
        &ledger,
        &snapshot::json_bytes(&rules).unwrap(),
        &Default::default(),
    )
    .unwrap();
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
        .find(|entry| entry.claim.as_deref() == Some(claim.id.as_str()))
        .unwrap();
    assert_eq!(entry.status, coverage::comparison::Status::MissingFromAtlas);
    assert_eq!(entry.gaps, vec!["reader evidence incomplete"]);
}

#[tokio::test]
async fn comparison_accepts_scalar_and_block_forms_without_unrelated_facet_gaps() {
    let source = r#"
types = { type[tradition] = { path = "game/common/traditions" } }
tradition = { unlocks_agenda = scalar on_enabled = {} }
"#;
    let ledger = ledger::inventory(&BTreeMap::from([("traditions.cwt".into(), source.into())]));
    let snapshot = recorded_snapshot().await;
    let bytes = snapshot::json_bytes(&snapshot).unwrap();
    let comparison = coverage::comparison::evaluate(&ledger, &bytes, &Default::default()).unwrap();
    for (property, subject) in [
        ("loader_path", vec!["types", "type[tradition]", "path"]),
        ("value_form", vec!["tradition", "unlocks_agenda"]),
        ("value_form", vec!["tradition", "on_enabled"]),
    ] {
        let claim = ledger
            .claims
            .iter()
            .find(|claim| claim.property == property && claim.subject == subject)
            .unwrap();
        let entry = comparison
            .entries
            .iter()
            .find(|entry| entry.claim.as_deref() == Some(claim.id.as_str()))
            .unwrap();
        assert_eq!(
            entry.status,
            coverage::comparison::Status::Same,
            "{}",
            claim.id
        );
        assert!(entry.gaps.is_empty());
    }
    let coverage = coverage::evaluate(&ledger, Some(&bytes)).unwrap();
    let block = ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "value_form" && claim.subject == ["tradition", "on_enabled"]
        })
        .unwrap();
    assert!(
        !coverage
            .claims
            .iter()
            .find(|assessment| assessment.claim == block.id)
            .unwrap()
            .covered
    );

    let alias_source = source.replace(
        "on_enabled = {}",
        "on_enabled = single_alias_right[effect_clause]",
    );
    let alias_ledger =
        ledger::inventory(&BTreeMap::from([("traditions.cwt".into(), alias_source)]));
    let alias_report =
        coverage::comparison::evaluate(&alias_ledger, &bytes, &Default::default()).unwrap();
    let alias_claim = alias_ledger
        .claims
        .iter()
        .find(|claim| {
            claim.property == "value_form" && claim.subject == ["tradition", "on_enabled"]
        })
        .unwrap();
    let alias_entry = alias_report
        .entries
        .iter()
        .find(|entry| entry.claim.as_deref() == Some(&alias_claim.id))
        .unwrap();
    assert_eq!(
        alias_entry.status,
        coverage::comparison::Status::MissingFromAtlas
    );
    assert!(!alias_entry.atlas_answers.is_empty());
}

#[tokio::test]
async fn shared_directory_keeps_native_and_mapping_gaps_together() {
    let source = r#"
types = {
    type[first] = { path = "game/common/traditions" }
    type[second] = { path = "game/common/traditions" }
}
first = { unlocks_agenda = scalar }
second = { unlocks_agenda = scalar }
"#;
    let ledger = ledger::inventory(&BTreeMap::from([("test.cwt".into(), source.into())]));
    let mut snapshot = recorded_snapshot().await;
    let id = "field:common/traditions/unlocks_agenda#value_form";
    snapshot.rules.retain(|rule| rule.id != id);
    snapshot.gaps.push(snapshot::Gap {
        id: id.into(),
        subject: "field:common/traditions/unlocks_agenda".into(),
        property: "value_form".into(),
        reason: "Native reader is unknown".into(),
        owner: None,
        native_gaps: vec![],
        evidence: vec![],
    });
    let comparison = coverage::comparison::evaluate(
        &ledger,
        &snapshot::json_bytes(&snapshot).unwrap(),
        &Default::default(),
    )
    .unwrap();
    for name in ["first", "second"] {
        let claim = ledger
            .claims
            .iter()
            .find(|claim| {
                claim.property == "value_form" && claim.subject == [name, "unlocks_agenda"]
            })
            .unwrap();
        let entry = comparison
            .entries
            .iter()
            .find(|entry| entry.claim.as_deref() == Some(&claim.id))
            .unwrap();
        assert!(
            entry
                .gaps
                .iter()
                .any(|gap| gap == "Native reader is unknown")
        );
        assert_eq!(
            entry
                .gaps
                .iter()
                .filter(|gap| gap.contains("the applicable type is unresolved"))
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn repeat_facts_answer_maximum_questions_without_publishing_engine_limits() {
    use coverage::comparison::Status;
    let original = recorded_snapshot().await;
    let subject = "field:common/traditions/unlocks_agenda";
    for (repeat, maximum, expected) in [
        ("Replace", "1", Status::Same),
        ("Accumulate", "inf", Status::Same),
        ("Merges", "inf", Status::Same),
        ("Replace", "inf", Status::Different),
        ("Accumulate", "1", Status::Different),
        ("Merges", "1", Status::Different),
    ] {
        let source = format!(
            "types = {{ type[tradition] = {{ path = \"game/common/traditions\" }} }}\ntradition = {{\n## cardinality = 0..{maximum}\nunlocks_agenda = scalar\n}}"
        );
        let ledger = ledger::inventory(&BTreeMap::from([("test.cwt".into(), source)]));
        let mut snapshot = original.clone();
        let evidence = snapshot
            .rules
            .iter()
            .find(|rule| rule.subject == subject && rule.property == "existence")
            .unwrap()
            .evidence
            .clone();
        snapshot
            .rules
            .retain(|rule| rule.id != format!("{subject}#repeat_behavior"));
        snapshot
            .gaps
            .retain(|gap| gap.id != format!("{subject}#occurrences.maximum"));
        snapshot.rules.push(snapshot::Rule {
            id: format!("{subject}#repeat_behavior"),
            subject: subject.into(),
            property: "repeat_behavior".into(),
            conditions: Vec::new(),
            answer: json!(repeat),
            evidence,
        });
        assert!(
            snapshot
                .rules
                .iter()
                .all(|rule| rule.property != "occurrences.maximum")
        );
        for qualified in [false, true] {
            if qualified {
                qualify_as_live(&mut snapshot);
            }
            let bytes = snapshot::json_bytes(&snapshot).unwrap();
            let report = coverage::evaluate(&ledger, Some(&bytes)).unwrap();
            let maximum = ledger
                .claims
                .iter()
                .find(|claim| claim.property == "cardinality_maximum")
                .unwrap();
            let minimum = ledger
                .claims
                .iter()
                .find(|claim| claim.property == "cardinality_minimum")
                .unwrap();
            assert_eq!(
                report
                    .claims
                    .iter()
                    .find(|claim| claim.claim == maximum.id)
                    .unwrap()
                    .covered,
                qualified
            );
            assert!(
                !report
                    .claims
                    .iter()
                    .find(|claim| claim.claim == minimum.id)
                    .unwrap()
                    .covered
            );
            if qualified {
                let comparison =
                    coverage::comparison::evaluate(&ledger, &bytes, &Default::default()).unwrap();
                let entry = comparison
                    .entries
                    .iter()
                    .find(|entry| entry.claim.as_deref() == Some(maximum.id.as_str()))
                    .unwrap();
                assert_eq!(entry.status, expected, "{repeat}");
            }
        }
    }
}
