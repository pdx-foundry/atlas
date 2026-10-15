use coverage::comparison::Status;
use pdx_atlas::{coverage, extraction, ledger, snapshot};
use pdx_native::{Basis, GameOptions, Native};
use serde_json::{Value, json};
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

        Self::from_snapshot(ledger, snapshot)
    }

    /// Evaluates the claims of `ledger` against `snapshot` qualified as live.
    fn from_snapshot(ledger: ledger::Ledger, mut snapshot: snapshot::Snapshot) -> Self {
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
        &self.entry(property, subject).gaps
    }

    /// The comparison status of the claim at `subject` with `property`.
    fn status(&self, property: &str, subject: &[&str]) -> coverage::comparison::Status {
        self.entry(property, subject).status
    }

    fn entry(&self, property: &str, subject: &[&str]) -> &coverage::comparison::Entry {
        let id = self.claim_id(property, subject);

        self.comparison
            .entries
            .iter()
            .find(|entry| entry.claim.as_deref() == Some(id))
            .unwrap()
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

    for filter in ["= named_lists", "= { named_lists other_lists }"] {
        let selected = species_names(filter).await;
        let species_name = ["types", "type[species_name]"];

        assert!(
            !selected.covered("type_existence", &species_name),
            "{filter}"
        );
        assert!(
            selected
                .gaps("type_existence", &species_name)
                .iter()
                .any(|gap| gap.contains(&format!("`type_key_filter {filter}`"))),
            "{filter}"
        );
    }
}

#[tokio::test]
async fn root_key_reaches_the_root_type_subtype_beside_a_secondary_type_with_the_same_key() {
    let source = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
        ## type_key_filter = special_tradition
        subtype[special] = {}
    }
    ## type_key_filter = other_swap
    type[swapped_tradition] = {
        base_type = tradition
        path = "game/common/traditions"
        skip_root_key = any
        ## type_key_filter = special_tradition
        subtype[special] = {}
    }
}
tradition = { subtype[special] = { unlocks_agenda = scalar } }
"#;
    let directory = SharedDirectory::evaluate(&[("traditions.cwt", source)], |snapshot| {
        let rule = snapshot
            .rules
            .iter_mut()
            .find(|rule| rule.id == "field:common/traditions/unlocks_agenda#existence")
            .unwrap();
        rule.conditions = vec!["root_key[special_tradition]".into()];
    })
    .await;

    assert!(directory.covered(
        "field_existence",
        &["tradition", "subtype[special]", "unlocks_agenda"]
    ));
}

async fn recorded_extraction() -> extraction::Extraction {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452");
    let native = Native::from_recorded_answers(path).unwrap();

    extraction::collect(&native, || GameOptions::new(Command::new("unused"))).await
}

async fn recorded_snapshot() -> snapshot::Snapshot {
    snapshot::assemble(&recorded_extraction().await).unwrap()
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

/// Adds a registry subject for `registry` with no rules.
fn add_registry(snapshot: &mut snapshot::Snapshot, registry: &str) {
    snapshot.subjects.push(snapshot::Subject {
        id: format!("registry:{registry}"),
        kind: snapshot::SubjectKind::Registry,
        registry: Some(registry.into()),
        field: None,
        conditional: None,
        name: None,
    });
}

/// Adds or replaces the rule `{subject}#{property}`, citing the traditions loader-rule evidence.
fn set_rule(snapshot: &mut snapshot::Snapshot, subject: &str, property: &str, answer: Value) {
    let template = snapshot
        .rules
        .iter()
        .find(|rule| rule.id == "registry:common/traditions#extension")
        .unwrap()
        .clone();
    let id = format!("{subject}#{property}");

    snapshot.rules.retain(|rule| rule.id != id);
    snapshot.rules.push(snapshot::Rule {
        id,
        subject: subject.into(),
        property: property.into(),
        answer,
        ..template
    });
}

/// Replaces the rule `{subject}#{property}` with a gap, as for an unresolved property.
fn unresolve(snapshot: &mut snapshot::Snapshot, subject: &str, property: &str) {
    let id = format!("{subject}#{property}");

    snapshot.rules.retain(|rule| rule.id != id);
    snapshot.gaps.push(snapshot::Gap {
        id,
        subject: subject.into(),
        property: property.into(),
        reason: format!("Native did not establish this registry's {property}"),
        owner: None,
        native_gaps: Vec::new(),
        evidence: Vec::new(),
    });
}

const TRADITIONS: &str = "registry:common/traditions";

#[tokio::test]
async fn file_selection_lines_join_the_loader_extension_and_recursion() {
    let source = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
        path_extension = .txt
        path_strict = yes
    }
}
"#;
    let extension = ["types", "type[tradition]", "path_extension"];
    let strict = ["types", "type[tradition]", "path_strict"];

    let recorded = SharedDirectory::evaluate(&[("traditions.cwt", source)], |_| {}).await;

    assert!(recorded.covered("loader_path", &extension));
    assert!(recorded.covered("loader_path", &strict));
    assert_eq!(
        recorded.status("loader_path", &extension),
        Status::Different
    );
    assert_eq!(recorded.status("loader_path", &strict), Status::Same);

    let edited = SharedDirectory::evaluate(&[("traditions.cwt", source)], |snapshot| {
        set_rule(
            snapshot,
            TRADITIONS,
            "extension",
            json!({ "suffix": ".txt" }),
        );
        set_rule(snapshot, TRADITIONS, "recursion", json!("Unlimited"));
    })
    .await;

    assert_eq!(edited.status("loader_path", &extension), Status::Same);
    assert_eq!(edited.status("loader_path", &strict), Status::Different);

    let levels = SharedDirectory::evaluate(&[("traditions.cwt", source)], |snapshot| {
        set_rule(snapshot, TRADITIONS, "recursion", json!({ "Levels": 2 }));
    })
    .await;

    assert_eq!(levels.status("loader_path", &strict), Status::Different);
}

#[tokio::test]
async fn override_priorities_join_the_duplicate_rule_of_their_directory() {
    let overrides = r#"
priorities = {
    "common/traditions" = lios
    "common/relics" = fios
    "common/buildings" = ordered
}
"#;
    let priority = |directory: &'static str| ["priorities", directory];

    let recorded = SharedDirectory::evaluate(&[("overrides.cwt", overrides)], |_| {}).await;

    for directory in ["common/traditions", "common/relics", "common/buildings"] {
        assert!(
            recorded.covered("value_form", &priority(directory)),
            "{directory}"
        );
        assert!(recorded.covered("field_existence", &priority(directory)));
        assert_eq!(
            recorded.status("field_existence", &priority(directory)),
            Status::Same
        );
    }
    assert_eq!(
        recorded.status("value_form", &priority("common/traditions")),
        Status::Same
    );
    assert_eq!(
        recorded.status("value_form", &priority("common/relics")),
        Status::Different
    );
    assert_eq!(
        recorded.status("value_form", &priority("common/buildings")),
        Status::Different
    );

    let merging = SharedDirectory::evaluate(&[("overrides.cwt", overrides)], |snapshot| {
        set_rule(
            snapshot,
            "registry:common/buildings",
            "duplicate_rule",
            json!("Merges"),
        );
    })
    .await;

    assert_eq!(
        merging.status("value_form", &priority("common/buildings")),
        Status::Same
    );
}

#[tokio::test]
async fn unresolved_loader_rules_credit_nothing_and_keep_their_gaps() {
    let types = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
        path_extension = .txt
        path_strict = yes
        name_field = name
    }
}
"#;
    let overrides = r#"priorities = { "common/traditions" = lios }"#;
    let directory = SharedDirectory::evaluate(
        &[("traditions.cwt", types), ("overrides.cwt", overrides)],
        |snapshot| {
            for property in [
                "extension",
                "recursion",
                "duplicate_rule",
                "definition_names",
            ] {
                unresolve(snapshot, TRADITIONS, property);
            }
        },
    )
    .await;

    for (property, subject, unresolved) in [
        (
            "loader_path",
            &["types", "type[tradition]", "path_extension"][..],
            "extension",
        ),
        (
            "loader_path",
            &["types", "type[tradition]", "path_strict"],
            "recursion",
        ),
        (
            "loader_path",
            &["types", "type[tradition]", "name_field"],
            "definition_names",
        ),
        (
            "value_form",
            &["priorities", "common/traditions"],
            "duplicate_rule",
        ),
    ] {
        assert!(!directory.covered(property, subject), "{subject:?}");
        assert!(
            directory
                .gaps(property, subject)
                .iter()
                .any(|gap| gap.contains(unresolved)),
            "{subject:?}"
        );
    }
}

#[tokio::test]
async fn naming_lines_join_the_form_of_their_type() {
    let traditions = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
        name_field = name
    }
    ## type_key_filter = tradition_swap
    type[swapped_tradition] = {
        base_type = tradition
        path = "game/common/traditions"
        skip_root_key = any
        name_field = name
    }
}
"#;
    let relics = r#"
types = {
    type[relic] = {
        path = "game/common/relics"
        skip_root_key = collection
        type_per_file = yes
        name_from_file = no
    }
}
"#;
    let directory = SharedDirectory::evaluate(
        &[("traditions.cwt", traditions), ("relics.cwt", relics)],
        |_| {},
    )
    .await;
    let swap = |line| ["types", "type[swapped_tradition]", line];
    let relic = |line| ["types", "type[relic]", line];

    for (subject, status) in [
        (
            ["types", "type[tradition]", "name_field"],
            Status::Different,
        ),
        (swap("name_field"), Status::Same),
        (swap("skip_root_key"), Status::Same),
        (relic("skip_root_key"), Status::Different),
        (relic("type_per_file"), Status::Different),
        (relic("name_from_file"), Status::Same),
    ] {
        assert!(directory.covered("loader_path", &subject), "{subject:?}");
        assert_eq!(
            directory.status("loader_path", &subject),
            status,
            "{subject:?}"
        );
    }
}

#[tokio::test]
async fn a_loader_that_names_no_definition_answers_only_the_name_field() {
    let relics = r#"
types = {
    type[relic] = {
        path = "game/common/relics"
        name_field = name
        skip_root_key = collection
        type_per_file = yes
    }
}
"#;
    let directory = SharedDirectory::evaluate(&[("relics.cwt", relics)], |snapshot| {
        set_rule(
            snapshot,
            "registry:common/relics",
            "definition_names",
            json!([]),
        );
    })
    .await;
    let relic = |line| ["types", "type[relic]", line];

    assert!(directory.covered("loader_path", &relic("name_field")));
    assert_eq!(
        directory.status("loader_path", &relic("name_field")),
        Status::Different
    );
    assert!(!directory.covered("loader_path", &relic("skip_root_key")));
    assert!(!directory.covered("loader_path", &relic("type_per_file")));
}

#[tokio::test]
async fn partial_names_credit_the_stated_root_form_and_publish_the_rest() {
    let jobs = r#"
types = {
    type[pop_job] = {
        path = "game/common/pop_jobs"
        name_field = name
    }
}
pop_job = {}
"#;
    let directory = SharedDirectory::evaluate(&[("jobs.cwt", jobs)], |_| {}).await;
    let name_field = ["types", "type[pop_job]", "name_field"];

    assert!(directory.covered("field_existence", &["pop_job"]));
    assert!(directory.covered("loader_path", &name_field));
    assert_eq!(
        directory.status("loader_path", &name_field),
        Status::Different
    );
    assert!(
        directory
            .coverage
            .atlas_only_questions
            .contains(&"atlas:registry:common/pop_jobs#definition_names.partial".into())
    );
}

/// A config `tradition` type with a definition line, its push scope, and one field.
const TRADITION_DEFINITION: &str = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
    }
}
## push_scope = country
tradition = {
    unlocks_agenda = scalar
}
"#;

#[tokio::test]
async fn a_root_form_credits_the_existence_of_the_definition_line_only() {
    let directory =
        SharedDirectory::evaluate(&[("traditions.cwt", TRADITION_DEFINITION)], |_| {}).await;
    let push_scope = ["tradition", "$annotation:push_scope"];

    assert!(directory.covered("field_existence", &["tradition"]));
    assert_eq!(
        directory.status("field_existence", &["tradition"]),
        Status::Same
    );
    assert!(!directory.covered("value_form", &["tradition"]));
    assert!(directory.gaps("value_form", &["tradition"]).contains(
        &"No Native answer states the value form of a root definition; a field of a definition \
          does not show that every definition is a block"
            .into()
    ));
    assert!(!directory.covered("scope_context", &push_scope));
    assert!(directory.gaps("scope_context", &push_scope).contains(
        &"Native states the scope of each block that a definition reads, on that block's field; \
          no answer states one scope for the whole definition"
            .into()
    ));
}

#[tokio::test]
async fn a_definition_line_without_a_root_form_stays_uncredited() {
    let unlisted = r#"
types = {
    type[relic] = {
        path = "game/common/not_a_registry"
    }
}
relic = {}
"#;
    let unlisted = SharedDirectory::evaluate(&[("relics.cwt", unlisted)], |_| {}).await;
    let unnamed = SharedDirectory::evaluate(&[("traditions.cwt", TRADITION_DEFINITION)], |s| {
        set_rule(s, TRADITIONS, "definition_names", json!([]));
    })
    .await;
    let unresolved =
        SharedDirectory::evaluate(&[("traditions.cwt", TRADITION_DEFINITION)], |snapshot| {
            unresolve(snapshot, TRADITIONS, "definition_names");
        })
        .await;

    assert!(!unlisted.covered("field_existence", &["relic"]));
    assert!(unlisted.gaps("value_form", &["relic"]).is_empty());
    assert!(!unnamed.covered("field_existence", &["tradition"]));
    assert!(!unresolved.covered("field_existence", &["tradition"]));
    assert!(
        unresolved
            .gaps("field_existence", &["tradition"])
            .contains(&"Native did not establish this registry's definition_names".into())
    );
}

#[tokio::test]
async fn a_root_key_form_joins_the_loader_lines_of_the_type_that_the_key_selects() {
    let events = r#"
types = {
    type[event] = {
        path = "game/events"
        name_field = id
    }
}
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
    let sources = [
        ("events/events.cwt", events),
        ("events/event_namespaces.cwt", namespaces),
    ];
    let forms = |namespace_name: Value| {
        json!([
            { "statement": { "RootKey": "namespace" }, "name": namespace_name },
            { "statement": "Root", "name": { "Field": "id" } },
        ])
    };
    let with_forms = |namespace_name: Value| {
        move |snapshot: &mut snapshot::Snapshot| {
            add_registry(snapshot, "events");
            set_rule(snapshot, "registry:events", "existence", json!(true));
            set_rule(
                snapshot,
                "registry:events",
                "definition_names",
                forms(namespace_name),
            );
        }
    };
    let namespace = ["types", "type[event_namespace]", "name_field"];
    let event = ["types", "type[event]", "name_field"];

    let agreeing = SharedDirectory::evaluate(&sources, with_forms(json!("Value"))).await;

    assert!(agreeing.covered("loader_path", &namespace));
    assert_eq!(agreeing.status("loader_path", &namespace), Status::Same);
    assert_eq!(agreeing.status("loader_path", &event), Status::Same);
    assert!(!agreeing.covered("type_existence", &["types", "type[event_namespace]"]));
    assert!(
        agreeing
            .gaps("type_existence", &["types", "type[event_namespace]"])
            .iter()
            .any(|gap| gap.contains("`type_key_filter = namespace`"))
    );
    assert!(!agreeing.covered("value_form", &["event_namespace"]));

    let differing = SharedDirectory::evaluate(&sources, with_forms(json!("Key"))).await;

    assert_eq!(
        differing.status("loader_path", &namespace),
        Status::Different
    );
}

#[tokio::test]
async fn path_file_joins_only_a_registry_named_by_the_one_file() {
    let alerts = r#"
types = {
    type[alert] = {
        path = "game/common/traditions"
        path_file = "alerts.txt"
    }
}
"#;
    let path_file = ["types", "type[alert]", "path_file"];

    let folder_only = SharedDirectory::evaluate(&[("alerts.cwt", alerts)], |_| {}).await;

    assert!(folder_only.covered("loader_path", &["types", "type[alert]", "path"]));
    assert!(!folder_only.covered("loader_path", &path_file));

    let single_file = SharedDirectory::evaluate(&[("alerts.cwt", alerts)], |snapshot| {
        let registry = "common/traditions/alerts.txt";

        add_registry(snapshot, registry);
        set_rule(
            snapshot,
            &format!("registry:{registry}"),
            "loader_path",
            json!(registry),
        );
    })
    .await;

    assert!(single_file.covered("loader_path", &path_file));
    assert_eq!(single_file.status("loader_path", &path_file), Status::Same);

    let shared = r#"
types = {
    type[alert] = {
        path = "game/common/traditions"
        path_file = "alerts.txt"
    }
    type[achievement] = {
        path = "game/common/traditions"
        path_file = "achievements.txt"
    }
}
"#;
    let shared_folder = SharedDirectory::evaluate(&[("alerts.cwt", shared)], |snapshot| {
        let registry = "common/traditions/alerts.txt";

        add_registry(snapshot, registry);
        set_rule(
            snapshot,
            &format!("registry:{registry}"),
            "loader_path",
            json!(registry),
        );
    })
    .await;

    assert!(shared_folder.covered("loader_path", &path_file));
    assert!(!shared_folder.covered("loader_path", &["types", "type[alert]", "path"]));
    assert!(!shared_folder.covered("loader_path", &["types", "type[achievement]", "path_file"]));
}

#[tokio::test]
async fn recorded_loader_rules_earn_no_credit_and_publish_the_build_wide_limits() {
    let overrides = r#"priorities = { "common/traditions" = lios }"#;
    let ledger = ledger::inventory(&BTreeMap::from([(
        "overrides.cwt".into(),
        overrides.into(),
    )]));
    let snapshot = recorded_snapshot().await;
    let report =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();

    assert_eq!(report.totals.atlas_owned.covered, 0);
    for limit in ["file_order", "mod_over_vanilla"] {
        assert!(
            report
                .atlas_only_questions
                .contains(&format!("atlas:inventory:loader_rules#{limit}"))
        );
    }
}

/// Event kinds and pre_trigger key sets as the config writes them, trimmed to the claims that the
/// event tests read.
const EVENT_CONFIG: &[(&str, &str)] = &[
    (
        "events/events.cwt",
        r#"
types = {
    type[event] = {
        path = "game/events"
        ## type_key_filter = event
        ## push_scope = any
        subtype[scopeless] = {}
        ## type_key_filter = country_event
        ## push_scope = country
        subtype[country] = {}
        ## type_key_filter = observer_event
        ## push_scope = country
        subtype[observer] = {}
        ## type_key_filter = carrier_event
        ## push_scope = carrier
        subtype[carrier] = {}
        ## type_key_filter = fleet_event
        ## push_scope = fleet
        subtype[fleet] = {}
    }
}
event = {
    subtype[country] = {
        pre_triggers = { alias_name[country_pre_trigger] = alias_match_left[country_pre_trigger] }
    }
    subtype[carrier] = {
        pre_triggers = { alias_name[colony_pre_trigger] = alias_match_left[colony_pre_trigger] }
    }
}
"#,
    ),
    (
        "common/pop_jobs.cwt",
        r#"
types = { type[job] = { path = "game/common/pop_jobs" } }
job = { possible_pre_triggers = { alias_name[pop_pre_trigger] = alias_match_left[pop_pre_trigger] } }
"#,
    ),
    (
        "pre_triggers.cwt",
        r#"
alias[colony_pre_trigger:has_owner] = bool
alias[colony_pre_trigger:is_capital] = bool
alias[pop_pre_trigger:is_enslaved] = bool
alias[pop_pre_trigger:is_robotic] = bool
alias[country_pre_trigger:is_ai] = bool
"#,
    ),
    (
        "scopes.cwt",
        r#"
scopes = {
    Country = { aliases = { country } }
    Carrier = { aliases = { carrier } }
    Fleet = { aliases = { fleet } }
}
"#,
    ),
];

/// Native's answers for the event kinds and the `common/pop_jobs` pre_trigger block, beside the
/// recorded M452 answers, against [`EVENT_CONFIG`].
async fn event_kinds() -> SharedDirectory {
    event_kinds_against(EVENT_CONFIG).await
}

/// [`event_kinds`] against the config `sources`.
async fn event_kinds_against(sources: &[(&str, &str)]) -> SharedDirectory {
    use pdx_native::{
        Field, FieldMembers, FieldReadOutcome, GrammarProperty, ReadScope, ReaderKind,
    };

    let mut extraction = recorded_extraction().await;
    let traditions = extraction.fields[extraction::TRADITIONS]
        .as_ref()
        .unwrap()
        .clone();
    let template = |name: &str| {
        traditions
            .value
            .iter()
            .find(|field| field.name == name)
            .unwrap()
            .clone()
    };
    let key = |name: &str, kind: Option<ReaderKind>| {
        let mut field = template("unlocks_agenda");
        field.name = name.into();
        field.read[0].outcome = match (kind, field.read[0].outcome.clone()) {
            (Some(kind), FieldReadOutcome::Read { mut reader, shape }) => {
                reader.kind = kind;
                reader.id = serde_json::from_value(json!(format!("{:016x}", kind as u8))).unwrap();
                field.reader = reader.clone();
                FieldReadOutcome::Read { reader, shape }
            }
            _ => FieldReadOutcome::Unresolved,
        };
        field
    };
    let block = |name: &str, read_scope, members| Field {
        name: name.into(),
        read_scope,
        members: FieldMembers::Fields(members),
        ..template("tradition_swap")
    };
    let scope = |id: &str| serde_json::from_value(json!({ "id": id, "name": "authored" })).unwrap();
    let read_in = |ids: &[&str]| {
        GrammarProperty::Known(vec![ReadScope::Types(
            ids.iter().map(|id| scope(id)).collect(),
        )])
    };
    let (country, observer, planet, ship) = (
        "44b5e21fe5e593d7",
        "c2bdd32188145ee1",
        "37c52c03d3012e74",
        "69ead02b278522d1",
    );
    let boolean = Some(ReaderKind::Boolean);
    let kinds = vec![
        block("event", GrammarProperty::Known(Vec::new()), Vec::new()),
        block(
            "country_event",
            read_in(&[country]),
            vec![block(
                "pre_triggers",
                GrammarProperty::Unresolved,
                vec![key("is_ai", boolean), key("is_idle", boolean)],
            )],
        ),
        block("observer_event", read_in(&[observer]), Vec::new()),
        block(
            "carrier_event",
            read_in(&[planet, ship]),
            vec![block(
                "pre_triggers",
                GrammarProperty::Unresolved,
                vec![key("has_owner", boolean), key("is_capital", None)],
            )],
        ),
        block("fleet_event", GrammarProperty::Unresolved, Vec::new()),
    ];
    let jobs = vec![block(
        "possible_pre_triggers",
        GrammarProperty::Unresolved,
        vec![key("is_enslaved", Some(ReaderKind::Integer))],
    )];

    for (registry, fields) in [("events", kinds), ("common/pop_jobs", jobs)] {
        let mut answer = traditions.clone();
        answer.value = fields;
        answer.gaps = Vec::new();
        extraction.fields.insert(registry.into(), Ok(answer));
    }

    // The events loader's two naming forms, as Native states them on M452.
    let mut loader_rules = extraction.loader_rules[extraction::TRADITIONS]
        .as_ref()
        .unwrap()
        .clone();
    loader_rules.value.definition_names = GrammarProperty::Known(
        serde_json::from_value(json!([
            { "statement": { "RootKey": "namespace" }, "name": "Value" },
            { "statement": "Root", "name": { "Field": "id" } },
        ]))
        .unwrap(),
    );
    extraction
        .loader_rules
        .insert("events".into(), Ok(loader_rules));

    let ledger = ledger::inventory(
        &sources
            .iter()
            .map(|(file, source)| ((*file).into(), (*source).into()))
            .collect(),
    );

    SharedDirectory::from_snapshot(ledger, snapshot::assemble(&extraction).unwrap())
}

fn push_scope(subtype: &str) -> [&str; 4] {
    ["types", "type[event]", subtype, "$annotation:push_scope"]
}

#[tokio::test]
async fn event_kind_scope_credits_its_subtype_push_scope_and_a_gap_does_not() {
    let events = event_kinds().await;

    assert!(events.covered("scope_context", &push_scope("subtype[country]")));
    assert!(events.covered("scope_context", &push_scope("subtype[scopeless]")));
    assert!(!events.covered("scope_context", &push_scope("subtype[fleet]")));
    assert!(
        events
            .gaps("scope_context", &push_scope("subtype[fleet]"))
            .iter()
            .any(|gap| gap.contains("did not establish the scope"))
    );
}

#[tokio::test]
async fn event_kind_child_credits_the_field_of_its_subtype() {
    let events = event_kinds().await;

    assert!(events.covered(
        "field_existence",
        &["event", "subtype[country]", "pre_triggers"]
    ));
}

#[tokio::test]
async fn event_kinds_credit_the_type_wide_definition_block() {
    let events = event_kinds().await;

    assert!(events.covered("field_existence", &["event"]));
    assert!(events.covered("value_form", &["event"]));
}

#[tokio::test]
async fn pre_trigger_keys_credit_their_alias_claims() {
    let events = event_kinds().await;

    for alias in [
        "alias[country_pre_trigger:is_ai]",
        "alias[colony_pre_trigger:has_owner]",
        "alias[pop_pre_trigger:is_enslaved]",
    ] {
        assert!(events.covered("value_form", &[alias]), "{alias}");
    }

    assert!(!events.covered("value_form", &["alias[pop_pre_trigger:is_robotic]"]));
    assert!(!events.covered("value_form", &["alias[colony_pre_trigger:is_capital]"]));
    assert!(
        !events
            .gaps("value_form", &["alias[colony_pre_trigger:is_capital]"])
            .is_empty()
    );
}

#[tokio::test]
async fn a_block_that_takes_only_alias_names_answers_no_alias_value() {
    let sources: Vec<_> = EVENT_CONFIG
        .iter()
        .map(|&(file, source)| match file {
            "common/pop_jobs.cwt" => (
                file,
                r#"
types = { type[job] = { path = "game/common/pop_jobs" } }
job = { possible_pre_triggers = { alias_name[pop_pre_trigger] = bool } }
"#,
            ),
            _ => (file, source),
        })
        .collect();
    let events = event_kinds_against(&sources).await;

    assert!(!events.covered("value_form", &["alias[pop_pre_trigger:is_enslaved]"]));
    assert!(events.covered("value_form", &["alias[country_pre_trigger:is_ai]"]));
}

#[tokio::test]
async fn event_kind_scopes_and_pre_trigger_keys_compare_with_the_config() {
    let events = event_kinds().await;

    for (subtype, status) in [
        ("subtype[country]", Status::Same),
        ("subtype[scopeless]", Status::Same),
        ("subtype[carrier]", Status::Same),
        ("subtype[observer]", Status::Different),
    ] {
        assert_eq!(
            events.status("scope_context", &push_scope(subtype)),
            status,
            "{subtype}"
        );
    }

    assert_eq!(
        events.status("value_form", &["alias[country_pre_trigger:is_ai]"]),
        Status::Same
    );
    assert_eq!(
        events.status("value_form", &["alias[pop_pre_trigger:is_enslaved]"]),
        Status::Different
    );
}

#[tokio::test]
async fn a_root_field_named_like_a_root_key_stays_a_field_outside_events() {
    let source = r#"
types = {
    type[tradition] = {
        path = "game/common/traditions"
        ## type_key_filter = tradition_swap
        subtype[swap] = {}
    }
}
tradition = {
    tradition_swap = { name = scalar }
    subtype[swap] = { name = scalar }
}
"#;
    let directory = SharedDirectory::evaluate(&[("traditions.cwt", source)], |_| {}).await;

    assert!(directory.covered("field_existence", &["tradition", "tradition_swap"]));
    assert!(!directory.covered("field_existence", &["tradition", "subtype[swap]", "name"]));
}

const PLANET_CLASS_TYPE: &str = r#"
    ## type_key_filter <> random_list
    type[planet_class] = { path = "game/common/planet_classes" }"#;
const RANDOM_LIST_TYPE: &str = r#"
    ## type_key_filter = random_list
    type[planet_class_random_list] = {
        path = "game/common/planet_classes"
        name_field = "name"
    }"#;
const PLANET_CLASS_BLOCKS: &str = r#"
planet_class = {
    can_be_capital = bool
    random_list = scalar
}
planet_class_random_list = {
    name = scalar
    can_be_capital = bool
    planets = { <planet_class> }
}
"#;

/// The recorded M452 answers, with `common/planet_classes` answering a planet-class field and the
/// root statement `random_list` with its two fields, as Native does (SDK-813), against the
/// planet-class config written by `types`.
async fn planet_classes(types: &str) -> SharedDirectory {
    use pdx_native::FieldMembers;

    let mut extraction = recorded_extraction().await;
    let traditions = extraction.fields[extraction::TRADITIONS]
        .as_ref()
        .unwrap()
        .clone();
    let template = |name: &str| {
        traditions
            .value
            .iter()
            .find(|field| field.name == name)
            .unwrap()
            .clone()
    };
    let key = |name: &str| pdx_native::Field {
        name: name.into(),
        ..template("unlocks_agenda")
    };
    let random_list = pdx_native::Field {
        name: "random_list".into(),
        members: FieldMembers::Fields(vec![key("name"), key("planets")]),
        ..template("tradition_swap")
    };
    let mut answer = traditions.clone();
    answer.value = vec![key("can_be_capital"), random_list];
    answer.gaps = Vec::new();
    extraction
        .fields
        .insert("common/planet_classes".into(), Ok(answer));

    let source = format!("types = {{{types}\n}}\n{PLANET_CLASS_BLOCKS}");
    let ledger = ledger::inventory(&BTreeMap::from([("planet_classes.cwt".into(), source)]));

    SharedDirectory::from_snapshot(ledger, snapshot::assemble(&extraction).unwrap())
}

#[tokio::test]
async fn a_root_statement_key_answers_the_type_that_it_selects() {
    let directory = planet_classes(&format!("{PLANET_CLASS_TYPE}{RANDOM_LIST_TYPE}")).await;

    for (property, subject) in [
        (
            "type_existence",
            &["types", "type[planet_class_random_list]"][..],
        ),
        ("field_existence", &["planet_class_random_list"]),
        ("value_form", &["planet_class_random_list"]),
        ("field_existence", &["planet_class_random_list", "name"]),
        ("field_existence", &["planet_class", "can_be_capital"]),
    ] {
        assert!(directory.covered(property, subject), "{subject:?}");
    }

    // A planet class has no `random_list` member, and a planet-class field is not a random-list
    // field.
    assert!(!directory.covered("field_existence", &["planet_class", "random_list"]));
    assert!(!directory.covered(
        "field_existence",
        &["planet_class_random_list", "can_be_capital"]
    ));
}

#[tokio::test]
async fn a_root_statement_type_alone_in_its_directory_keeps_its_own_fields() {
    let directory = planet_classes(RANDOM_LIST_TYPE).await;

    assert!(directory.covered("field_existence", &["planet_class_random_list", "name"]));
    assert!(!directory.covered(
        "field_existence",
        &["planet_class_random_list", "can_be_capital"]
    ));
}

#[tokio::test]
async fn a_root_statement_is_no_field_of_the_root_type_without_its_own_type() {
    let directory = planet_classes(PLANET_CLASS_TYPE).await;

    assert!(directory.covered("field_existence", &["planet_class", "can_be_capital"]));
    assert!(!directory.covered("field_existence", &["planet_class", "random_list"]));
}

const AGENDA_KINDS: &str = r#"
types = { type[tradition] = { path = "game/common/traditions" } }
enums = { enum[agenda_kind] = { first second } }
tradition = { unlocks_agenda = enum[agenda_kind] }
"#;

/// The traditions config with `unlocks_agenda = enum[agenda_kind]`, against the recorded answers
/// changed by `edit` and qualified as live.
async fn agenda_kinds(edit: impl FnOnce(&mut snapshot::Snapshot)) -> SharedDirectory {
    SharedDirectory::evaluate(&[("traditions.cwt", AGENDA_KINDS)], edit).await
}

const UNLOCKS_AGENDA: &str = "field:common/traditions/unlocks_agenda";

/// Replaces the gap `{subject}#{property}` with a rule that answers `answer`.
fn establish(snapshot: &mut snapshot::Snapshot, subject: &str, property: &str, answer: Value) {
    let id = format!("{subject}#{property}");

    snapshot.gaps.retain(|gap| gap.id != id);
    set_rule(snapshot, subject, property, answer);
}

fn agenda_kind(item: &str) -> [&str; 3] {
    ["enums", "enum[agenda_kind]", item]
}

#[tokio::test]
async fn a_listed_domain_answers_each_member_of_the_fields_enum() {
    let directory = agenda_kinds(|snapshot| {
        establish(
            snapshot,
            UNLOCKS_AGENDA,
            "domain",
            json!(["first", "third"]),
        );
    })
    .await;

    assert!(directory.covered("enum_member", &agenda_kind("$item:1")));
    assert!(directory.covered("enum_member", &agenda_kind("$item:2")));
    assert_eq!(
        directory.status("enum_member", &agenda_kind("$item:1")),
        Status::Same
    );
    assert_eq!(
        directory.status("enum_member", &agenda_kind("$item:2")),
        Status::Different
    );
}

#[tokio::test]
async fn a_field_of_an_unidentified_type_does_not_answer_its_enum() {
    let source = r#"
types = {
    type[tradition] = { path = "game/common/traditions" }
    ## type_key_filter = unknown_key
    type[special_tradition] = { path = "game/common/traditions" }
}
enums = { enum[agenda_kind] = { first second } }
tradition = { unlocks_agenda = scalar }
special_tradition = { unlocks_agenda = enum[agenda_kind] }
"#;
    let directory = SharedDirectory::evaluate(&[("traditions.cwt", source)], |snapshot| {
        establish(snapshot, UNLOCKS_AGENDA, "domain", json!(["first"]));
    })
    .await;

    assert!(!directory.covered("enum_member", &agenda_kind("$item:1")));
    assert!(
        directory
            .gaps("enum_member", &agenda_kind("$item:1"))
            .iter()
            .any(|gap| gap.contains("type_key_filter = unknown_key"))
    );
}

#[tokio::test]
async fn an_unknown_domain_keeps_a_gap_on_the_fields_enum() {
    let directory = agenda_kinds(|_| {}).await;

    assert!(!directory.covered("enum_member", &agenda_kind("$item:1")));
    assert!(
        !directory
            .gaps("enum_member", &agenda_kind("$item:1"))
            .is_empty()
    );
}

/// The traditions config with `unlocks_agenda` at `cardinality`, against the recorded answers
/// with the field's requirement as `minimum`, or its gap when `None`.
async fn required_agenda(cardinality: &str, minimum: Option<u64>) -> SharedDirectory {
    let source = format!(
        r#"
types = {{ type[tradition] = {{ path = "game/common/traditions" }} }}
tradition = {{
    ## cardinality = {cardinality}
    unlocks_agenda = scalar
}}
"#
    );

    SharedDirectory::evaluate(&[("traditions.cwt", &source)], |snapshot| {
        let id = format!("{UNLOCKS_AGENDA}#occurrences.minimum");

        match minimum {
            Some(minimum) => establish(
                snapshot,
                UNLOCKS_AGENDA,
                "occurrences.minimum",
                json!(minimum),
            ),
            None => {
                snapshot.rules.retain(|rule| rule.id != id);
                assert!(snapshot.gaps.iter().any(|gap| gap.id == id));
            }
        }
    })
    .await
}

const AGENDA_CARDINALITY: [&str; 3] = ["tradition", "unlocks_agenda", "$annotation:cardinality"];

#[tokio::test]
async fn a_requirement_answers_a_minimum_of_zero_or_one() {
    for (cardinality, minimum, status) in [
        ("1..1", 1, Status::Same),
        ("0..1", 0, Status::Same),
        ("0..1", 1, Status::Different),
        ("1..1", 0, Status::Different),
        ("2..inf", 0, Status::Different),
    ] {
        let directory = required_agenda(cardinality, Some(minimum)).await;

        assert!(
            directory.covered("cardinality_minimum", &AGENDA_CARDINALITY),
            "{cardinality} {minimum}"
        );
        assert_eq!(
            directory.status("cardinality_minimum", &AGENDA_CARDINALITY),
            status,
            "{cardinality} {minimum}"
        );
    }
}

#[tokio::test]
async fn a_required_field_does_not_answer_a_count_above_one() {
    let directory = required_agenda("2..inf", Some(1)).await;

    assert!(!directory.covered("cardinality_minimum", &AGENDA_CARDINALITY));
    assert!(
        directory
            .gaps("cardinality_minimum", &AGENDA_CARDINALITY)
            .iter()
            .any(|gap| gap.contains("not a count above 1"))
    );
}

#[tokio::test]
async fn an_unknown_requirement_keeps_the_minimum_gap() {
    let directory = required_agenda("1..1", None).await;

    assert!(!directory.covered("cardinality_minimum", &AGENDA_CARDINALITY));
}
