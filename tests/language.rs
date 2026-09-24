use pdx_atlas::{
    coverage::{self, comparison},
    extraction::{self, Extraction},
    ledger,
    snapshot::{self, Snapshot, SubjectKind},
};
use pdx_native::{
    Basis, Disposal, GameOptions, Gap, GapKind, LocalizationOutput, Native, OnAction,
    ScopeDeclaration,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, process::Command};

fn recording() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m45")
}

async fn recorded() -> Extraction {
    let native = Native::from_recorded_answers(recording()).unwrap();
    extraction::collect(&native, || GameOptions::new(Command::new("unused"))).await
}

fn rule<'a>(snapshot: &'a Snapshot, id: &str) -> Option<&'a snapshot::Rule> {
    snapshot.rules.iter().find(|rule| rule.id == id)
}

fn gap<'a>(snapshot: &'a Snapshot, id: &str) -> Option<&'a snapshot::Gap> {
    snapshot.gaps.iter().find(|gap| gap.id == id)
}

fn qualify_as_live(snapshot: &mut Snapshot) {
    let mut keys = BTreeMap::new();

    for (old, mut source) in std::mem::take(&mut snapshot.sources) {
        source.basis = Basis::StaticAnalysis;
        let new = format!("{}@static_analysis", source.method);

        keys.insert(old, new.clone());
        snapshot.sources.insert(new, source);
    }

    for answer in snapshot.answers.values_mut() {
        answer.source = keys[&answer.source].clone();
    }
}

#[tokio::test]
async fn every_language_question_has_recorded_answers() {
    let snapshot = snapshot::assemble(&recorded().await).unwrap();

    for question in [
        "effects",
        "triggers",
        "modifiers",
        "modifier_categories",
        "scopes",
        "scope_links",
        "localization",
        "on_actions",
        "game_rules",
        "defines",
        "loaded_modifiers",
    ] {
        let id = SubjectKind::Inventory.id(question);

        assert!(
            snapshot.subjects.iter().any(|subject| subject.id == id),
            "{id}"
        );
        assert!(gap(&snapshot, &format!("{id}#answer")).is_none(), "{id}");
    }

    for (id, property) in [
        ("effect:add_age", "existence"),
        ("trigger:has_modifier", "existence"),
        ("modifier:pop_happiness", "category_tags"),
        ("modifier_category:Countries", "existence"),
        (
            "modifier_family:common/buildings/planet_{key}_build_speed_mult",
            "generation",
        ),
        ("scope:country/country", "keywords"),
        ("scope_group:carrier", "members"),
        ("scope_link:owner", "output_scope"),
        ("localization_command:GetName", "contexts"),
        ("on_action:on_game_start", "existence"),
        ("game_rule:can_have_robot_pops", "kind"),
        ("define:NGameplay.LOGISTIC_CEILING_MIN", "value_type"),
    ] {
        let rule = rule(&snapshot, &format!("{id}#{property}"))
            .unwrap_or_else(|| panic!("{id}#{property}"));

        for evidence in &rule.evidence {
            let source = &snapshot.sources[&snapshot.answers[&evidence.answer].source];

            assert_eq!(source.basis, Basis::Recorded);
        }
    }

    assert_eq!(
        rule(&snapshot, "effect:add_age#documentation")
            .unwrap()
            .answer["text_origin"],
        "engine"
    );
    assert_eq!(
        gap(&snapshot, "effect:add_age#arguments")
            .unwrap()
            .owner
            .as_deref(),
        Some("SDK-548")
    );
}

#[tokio::test]
async fn partial_answers_never_give_completeness_rules() {
    let extraction = recorded().await;
    let snapshot = snapshot::assemble(&extraction).unwrap();

    assert_eq!(
        extraction.language.effects.as_ref().unwrap().completeness,
        pdx_native::Completeness::Partial
    );
    assert!(
        snapshot
            .rules
            .iter()
            .filter(|rule| rule.subject.starts_with("inventory:"))
            .all(|rule| rule.property == "loaded_summary" && !rule.conditions.is_empty())
    );
    assert!(gap(&snapshot, "effect:set_variable#declared_scopes").is_some());
    assert!(rule(&snapshot, "effect:set_variable#declared_scopes").is_none());
    assert!(rule(&snapshot, "effect:set_variable#existence").is_some());
}

#[tokio::test]
async fn each_native_gap_becomes_a_snapshot_gap() {
    let extraction = recorded().await;
    let snapshot = snapshot::assemble(&extraction).unwrap();
    let mut recorded = BTreeMap::new();

    for gap in snapshot
        .gaps
        .iter()
        .filter(|gap| gap.property.starts_with("native."))
    {
        assert!(!gap.reason.is_empty());
        *recorded.entry(gap.evidence[0].answer.clone()).or_insert(0) += gap.native_gaps.len();
    }

    let language = &extraction.language;
    let expected = [
        (
            "declarations/effect",
            language.effects.as_ref().unwrap().gaps.len(),
        ),
        (
            "declarations/trigger",
            language.triggers.as_ref().unwrap().gaps.len(),
        ),
        ("modifiers", language.modifiers.as_ref().unwrap().gaps.len()),
        (
            "scope_links",
            language.scope_links.as_ref().unwrap().gaps.len(),
        ),
        (
            "localization_declarations",
            language.localization.as_ref().unwrap().gaps.len(),
        ),
        (
            "on_actions",
            language.on_actions.as_ref().unwrap().gaps.len(),
        ),
        (
            "game_rules",
            language.game_rules.as_ref().unwrap().gaps.len(),
        ),
        ("defines", language.defines.as_ref().unwrap().gaps.len()),
        (
            "modifier_families/common/buildings",
            language.modifier_families["common/buildings"]
                .as_ref()
                .unwrap()
                .gaps
                .len(),
        ),
    ];

    for (answer, count) in expected {
        assert!(count > 0, "{answer}");
        assert_eq!(recorded.get(answer), Some(&count), "{answer}");
    }

    let unresolved = snapshot
        .gaps
        .iter()
        .find(|gap| {
            gap.id == "define:NAI.VOIDWORMS_NAVAL_CAP_PER_FLEET#native.defines.unresolved_reader"
        })
        .unwrap();
    assert_eq!(unresolved.native_gaps[0].kind, GapKind::UnresolvedReader);
    assert!(
        rule(
            &snapshot,
            "define:NAI.VOIDWORMS_NAVAL_CAP_PER_FLEET#existence"
        )
        .is_none()
    );
}

#[tokio::test]
async fn entry_scopes_need_every_slot_established() {
    let mut extraction = recorded().await;
    let scopes = &extraction.language.scopes.as_ref().unwrap().value;
    let country = scopes
        .types
        .iter()
        .find(|scope| scope.name == "country" && scope.keywords == ["country"])
        .unwrap();
    let reference = json!({"Scope": {"id": country.id, "name": "country"}});
    let on_actions: Vec<OnAction> = serde_json::from_value(json!([
        {"name": "atlas_resolved", "entries": [
            {"this": reference, "root": reference, "from": ["NotSet"]}
        ]},
        {"name": "atlas_self_link", "entries": [
            {"this": reference, "root": "SelfLink", "from": []}
        ]},
        {"name": "atlas_unresolved_from", "entries": [
            {"this": reference, "root": reference, "from": ["Unresolved"]}
        ]},
        {"name": "atlas_no_site", "entries": []}
    ]))
    .unwrap();
    extraction.language.on_actions.as_mut().unwrap().value = on_actions;
    let snapshot = snapshot::assemble(&extraction).unwrap();

    assert_eq!(
        rule(&snapshot, "on_action:atlas_resolved#entry_scopes")
            .unwrap()
            .answer,
        json!([{"this": "scope:country/country", "root": "scope:country/country", "from": ["not_set"]}])
    );

    for name in ["atlas_self_link", "atlas_unresolved_from", "atlas_no_site"] {
        let id = format!("on_action:{name}#entry_scopes");

        assert!(rule(&snapshot, &id).is_none(), "{id}");
        assert_eq!(
            gap(&snapshot, &id).unwrap().owner.as_deref(),
            Some("SDK-496")
        );
        assert!(rule(&snapshot, &format!("on_action:{name}#existence")).is_some());
    }
}

#[tokio::test]
async fn unreadable_localization_rows_leave_context_lists_open() {
    let mut extraction = recorded().await;
    let complete = snapshot::assemble(&extraction).unwrap();

    assert!(rule(&complete, "localization_command:GetName#contexts").is_some());
    assert!(rule(&complete, "localization_command:GetName#scopes").is_some());

    extraction
        .language
        .localization
        .as_mut()
        .unwrap()
        .gaps
        .push(Gap {
            kind: GapKind::UnreadableInput,
            subject: Some("Country".into()),
            detail: "the command rows of this context could not be read (test)".into(),
        });
    let snapshot = snapshot::assemble(&extraction).unwrap();

    for property in ["contexts", "scopes"] {
        let id = format!("localization_command:GetName#{property}");

        assert!(rule(&snapshot, &id).is_none(), "{id}");
        assert!(gap(&snapshot, &id).is_some(), "{id}");
    }
    assert!(rule(&snapshot, "localization_command:GetName#existence").is_some());
}

#[tokio::test]
async fn localization_links_keep_each_input_with_its_output() {
    let extraction = recorded().await;
    let snapshot = snapshot::assemble(&extraction).unwrap();
    let declarations = &extraction.language.localization.as_ref().unwrap().value;
    let rows = |name: &str| {
        declarations
            .links
            .iter()
            .filter(|link| link.name == name)
            .collect::<Vec<_>>()
    };

    let leader = rule(&snapshot, "localization_link:Leader#alternatives").unwrap();
    assert_eq!(
        leader.answer.as_array().unwrap().len(),
        rows("Leader").len()
    );
    assert!(leader.answer.as_array().unwrap().len() > 1);

    let mixed = declarations
        .links
        .iter()
        .map(|link| link.name.as_str())
        .find(|name| {
            let rows = rows(name);

            rows.iter()
                .any(|row| row.output == LocalizationOutput::Unresolved)
                && rows
                    .iter()
                    .any(|row| row.output != LocalizationOutput::Unresolved)
        })
        .unwrap();
    assert!(
        rule(
            &snapshot,
            &format!("localization_link:{mixed}#alternatives")
        )
        .is_none()
    );
    assert!(
        gap(
            &snapshot,
            &format!("localization_link:{mixed}#alternatives")
        )
        .is_some()
    );
}

#[tokio::test]
async fn loaded_modifier_names_stay_out_of_the_snapshot() {
    let mut extraction = recorded().await;
    let snapshot = snapshot::assemble(&extraction).unwrap();
    let summary = rule(&snapshot, "inventory:loaded_modifiers#loaded_summary").unwrap();

    assert_eq!(summary.conditions, ["content:installation"]);
    assert_eq!(
        summary.answer,
        json!({"total": 4, "declared": 2, "generated": 1, "unexplained": 1})
    );
    assert!(
        snapshot
            .subjects
            .iter()
            .all(|subject| subject.id != "modifier:job_miner_add")
    );

    extraction.loaded_modifiers.disposal = Ok(Disposal::Unconfirmed("test".into()));
    let undisposed = snapshot::assemble(&extraction).unwrap();

    assert!(rule(&undisposed, "inventory:loaded_modifiers#loaded_summary").is_none());
    assert!(gap(&undisposed, "inventory:loaded_modifiers#answer").is_some());
}

#[tokio::test]
async fn scope_identity_does_not_depend_on_other_scopes() {
    let mut extraction = recorded().await;
    let before = snapshot::assemble(&extraction).unwrap();
    let ids = |snapshot: &Snapshot| {
        snapshot
            .subjects
            .iter()
            .filter(|subject| subject.kind == SubjectKind::Scope)
            .map(|subject| subject.id.clone())
            .collect::<Vec<_>>()
    };

    assert!(ids(&before).contains(&"scope:country/country".to_owned()));
    assert!(ids(&before).contains(&"scope:country/observer".to_owned()));

    let scopes = &mut extraction.language.scopes.as_mut().unwrap().value;
    let mut added: ScopeDeclaration = scopes.types[0].clone();
    added.name = "country".into();
    added.keywords = vec!["atlas_test".into()];
    let mut id = serde_json::to_value(&added.id).unwrap();
    id = json!(format!("{}0", id.as_str().unwrap()));
    added.id = serde_json::from_value(id).unwrap();
    scopes.types.push(added);
    let after = snapshot::assemble(&extraction).unwrap();

    for id in ids(&before) {
        assert!(ids(&after).contains(&id), "{id}");
    }
    assert!(ids(&after).contains(&"scope:country/atlas_test".to_owned()));
}

#[tokio::test]
async fn unresolved_family_tags_and_conditions_are_gaps() {
    let snapshot = snapshot::assemble(&recorded().await).unwrap();
    let economic = "modifier_family:common/economic_categories/{key}_cost_mult";

    assert!(gap(&snapshot, &format!("{economic}#category_tags")).is_some());
    assert!(gap(&snapshot, &format!("{economic}#generation")).is_some());
    assert_eq!(
        rule(
            &snapshot,
            "modifier_family:common/buildings/planet_{key}_build_speed_mult#generation"
        )
        .unwrap()
        .answer,
        "every_item"
    );
    assert_eq!(
        rule(
            &snapshot,
            "modifier_family:common/buildings/planet_{key}_build_speed_mult#name_template"
        )
        .unwrap()
        .answer,
        json!([{"literal": "planet_"}, "item_key", {"literal": "_build_speed_mult"}])
    );
}

fn language_ledger() -> ledger::Ledger {
    let files = [
        (
            "effects.cwt",
            r#"
### Adds the age of the scoped leader
## scopes = { leader }
alias[effect:add_age] = value_field

alias[effect:add_building] = {
    building = <building>
}
"#,
        ),
        (
            "scopes.cwt",
            r#"
scopes = {
    System = {
        aliases = { galacticobject system galactic_object }
    }
    Carrier = {
        aliases = { carrier }
    }
    "Pop Job" = {
        aliases = { job pop_job }
    }
}
"#,
        ),
        (
            "links.cwt",
            r#"
links = {
    owner = {
        input_scopes = { planet }
        output_scope = country
    }
}
"#,
        ),
        (
            "on_actions.cwt",
            r#"
on_actions = {
    ## replace_scopes = { this = no_scope root = no_scope }
    on_game_start
    on_atlas_unknown
}
"#,
        ),
        (
            "game_rules.cwt",
            r#"
game_rules = {
    ## replace_scopes = { this = country root = country }
    can_have_robot_pops = {
        alias_name[trigger] = alias_match_left[trigger]
    }
}
"#,
        ),
        (
            "common/defines/00_defines.cwt",
            r#"
defines = {
    NGameplay = {
        LOGISTIC_CEILING_MIN = int
    }
}
"#,
        ),
        (
            "modifiers.cwt",
            r#"
modifiers = {
    pop_happiness = { Pops }
}
"#,
        ),
        (
            "localisation.cwt",
            r#"
localisation_commands = {
    GetAdj = { country }
}
"#,
        ),
    ];

    ledger::inventory(
        &files
            .into_iter()
            .map(|(file, source)| (file.to_owned(), source.to_owned()))
            .collect(),
    )
}

fn claim<'a>(
    ledger: &'a ledger::Ledger,
    file: &str,
    subject: &[&str],
    property: &str,
) -> &'a ledger::Claim {
    ledger
        .claims
        .iter()
        .find(|claim| claim.file == file && claim.subject == subject && claim.property == property)
        .unwrap_or_else(|| panic!("{file} {subject:?} {property}"))
}

#[tokio::test]
async fn language_claims_join_their_snapshot_answers() {
    let ledger = language_ledger();
    let mut snapshot = snapshot::assemble(&recorded().await).unwrap();
    let recorded =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();

    assert_eq!(recorded.totals.atlas_owned.covered, 0);

    qualify_as_live(&mut snapshot);
    let report =
        coverage::evaluate(&ledger, Some(&snapshot::json_bytes(&snapshot).unwrap())).unwrap();
    let assessment = |claim: &ledger::Claim| {
        report
            .claims
            .iter()
            .find(|assessment| assessment.claim == claim.id)
            .unwrap()
            .clone()
    };

    for (file, subject, property) in [
        (
            "effects.cwt",
            vec!["alias[effect:add_age]"],
            "command_existence",
        ),
        (
            "effects.cwt",
            vec!["alias[effect:add_age]", "$annotation:scopes"],
            "declared_scopes",
        ),
        (
            "effects.cwt",
            vec!["alias[effect:add_age]"],
            "documentation",
        ),
        (
            "scopes.cwt",
            vec!["scopes", "System"],
            "declaration_existence",
        ),
        (
            "scopes.cwt",
            vec!["scopes", "Carrier"],
            "declaration_existence",
        ),
        ("links.cwt", vec!["links", "owner"], "declaration_existence"),
        (
            "links.cwt",
            vec!["links", "owner", "output_scope"],
            "declared_scopes",
        ),
        (
            "on_actions.cwt",
            vec!["on_actions", "$item:1"],
            "value_form",
        ),
        (
            "game_rules.cwt",
            vec!["game_rules", "can_have_robot_pops"],
            "field_existence",
        ),
        (
            "common/defines/00_defines.cwt",
            vec!["defines", "NGameplay", "LOGISTIC_CEILING_MIN"],
            "field_existence",
        ),
        (
            "common/defines/00_defines.cwt",
            vec!["defines", "NGameplay", "LOGISTIC_CEILING_MIN"],
            "value_form",
        ),
        (
            "modifiers.cwt",
            vec!["modifiers", "pop_happiness"],
            "declaration_existence",
        ),
        (
            "modifiers.cwt",
            vec!["modifiers", "pop_happiness", "$item:1"],
            "modifier_category",
        ),
        (
            "localisation.cwt",
            vec!["localisation_commands", "GetAdj"],
            "field_existence",
        ),
        (
            "localisation.cwt",
            vec!["localisation_commands", "GetAdj", "$item:1"],
            "value_form",
        ),
    ] {
        let assessment = assessment(claim(&ledger, file, &subject, property));

        assert!(
            assessment.covered,
            "{file} {subject:?} {property}: {}",
            assessment.reason
        );
    }

    for (file, subject, property, reason) in [
        (
            "effects.cwt",
            vec!["alias[effect:add_building]", "building"],
            "field_existence",
            "gap",
        ),
        (
            "on_actions.cwt",
            vec!["on_actions", "$item:1", "$annotation:replace_scopes"],
            "scope_context",
            "gap",
        ),
        (
            "game_rules.cwt",
            vec!["game_rules", "can_have_robot_pops"],
            "value_form",
            "No qualified answer",
        ),
        (
            "scopes.cwt",
            vec!["scopes", "Pop Job"],
            "declaration_existence",
            "No qualified answer",
        ),
        (
            "on_actions.cwt",
            vec!["on_actions", "$item:2"],
            "value_form",
            "No qualified answer",
        ),
    ] {
        let assessment = assessment(claim(&ledger, file, &subject, property));

        assert!(!assessment.covered, "{file} {subject:?} {property}");
        assert!(assessment.reason.contains(reason), "{}", assessment.reason);
    }
}

/// The recorded loaded table, trimmed to the entries these tests use (see the fixture README).
async fn loaded() -> pdx_native::Answer<pdx_native::LoadedModifiers> {
    let native = Native::from_recorded_answers(recording()).unwrap();
    extraction::read_loaded_modifiers(&native, GameOptions::new(Command::new("unused")))
        .await
        .observation
        .unwrap()
}

fn list<'a>(lists: &'a [comparison::NameList], name: &str) -> &'a comparison::NameList {
    lists.iter().find(|list| list.list == name).unwrap()
}

#[tokio::test]
async fn comparison_reports_logs_lists_defines_and_tags() {
    let ledger = language_ledger();
    let bytes = snapshot::json_bytes(&snapshot::assemble(&recorded().await).unwrap()).unwrap();
    let loaded = loaded().await;
    let logs = [
        (
            "effects.log",
            "[x]:\n== EFFECT DOCUMENTATION ==\nadd_age - Adds the age of the scoped leader\nadd_age = <int>\nSupported Scopes: leader\n\natlas_fake - Not an effect\nSupported Scopes: all\n",
        ),
        (
            "scopes.log",
            "[x]:\n== SCOPE DOCUMENTATION ==\nprose\nComplete list of scope changes:\n\nowner - Scopes to the owner\nSupported Scopes: planet\nOutput Scope: country\n",
        ),
        (
            "localizations.log",
            "--Country--\nPromotions:\n Capital\nProperties\n GetName\n GetAtlasFake\n",
        ),
        (
            "modifiers.log",
            "- pop_happiness, Category: Pops\n- atlas_fake_mult, Category: Countries\n",
        ),
    ];
    let inputs = comparison::Inputs {
        script_docs: logs
            .into_iter()
            .map(|(log, text)| (log.to_owned(), text.to_owned()))
            .collect(),
        define_files: BTreeMap::from([(
            "00_defines.txt".to_owned(),
            "NGameplay = {\n\tLOGISTIC_CEILING_MIN = 1 # comment\n\tATLAS_FAKE = 2\n}\n".to_owned(),
        )]),
        loaded_modifiers: Some(&loaded),
    };
    let report = comparison::evaluate(&ledger, &bytes, &inputs).unwrap();
    let log = |name: &str| {
        &report
            .script_docs
            .iter()
            .find(|log| log.log == name)
            .unwrap()
            .lists
    };

    let effects = list(log("effects.log"), "names");
    assert!(effects.agree.contains(&"add_age".to_owned()));
    assert_eq!(effects.config_only, ["atlas_fake"]);
    assert!(effects.engine_only.len() > 1000);
    assert_eq!(
        list(log("effects.log"), "supported_scopes").agree,
        ["add_age: leader"]
    );
    assert_eq!(
        list(log("effects.log"), "descriptions").agree,
        ["add_age: Adds the age of the scoped leader"]
    );
    assert!(
        list(log("scopes.log"), "names")
            .agree
            .contains(&"owner".to_owned())
    );
    assert_eq!(
        list(log("scopes.log"), "output_scope").agree,
        ["owner: country"]
    );
    assert_eq!(
        list(log("localizations.log"), "links").agree,
        ["Country.Capital"]
    );
    assert!(
        list(log("localizations.log"), "commands")
            .agree
            .contains(&"Country.GetName".to_owned())
    );
    assert_eq!(
        list(log("modifiers.log"), "names").config_only,
        ["atlas_fake_mult"]
    );
    assert_eq!(
        list(log("modifiers.log"), "categories").agree,
        ["pop_happiness: Pops"]
    );

    let on_actions = list(&report.name_lists, "on_actions");
    assert_eq!(on_actions.agree, ["on_game_start"]);
    assert_eq!(on_actions.config_only, ["on_atlas_unknown"]);
    let modifiers = list(&report.name_lists, "modifiers");
    assert!(modifiers.engine_only.contains(&"job_miner_add".to_owned()));

    let defines = report.define_files.as_ref().unwrap();
    assert!(
        defines
            .agree
            .contains(&"NGameplay.LOGISTIC_CEILING_MIN".to_owned())
    );
    assert_eq!(defines.config_only, ["NGameplay.ATLAS_FAKE"]);

    let tags = report.modifier_tags.as_ref().unwrap();
    let difference = |name: &str| {
        tags.different
            .iter()
            .find(|difference| difference.name == name)
            .unwrap()
    };
    assert_eq!(tags.agree, 1);
    assert_eq!(
        difference("gdf_ship_alloys_cost_mult").declared,
        Some(vec!["Countries".to_owned()])
    );
    assert!(difference("gdf_ship_alloys_cost_mult").loaded.is_some());
    assert_eq!(difference("blank_modifier").loaded, None);
    assert!(report.loaded_modifiers_read);
}

#[tokio::test]
async fn loaded_answer_from_another_build_is_refused() {
    let ledger = language_ledger();
    let bytes = snapshot::json_bytes(&snapshot::assemble(&recorded().await).unwrap()).unwrap();
    let mut loaded = loaded().await;
    let mut source: Value = serde_json::to_value(&loaded.source).unwrap();
    source["build"] = json!("another-build");
    loaded.source = serde_json::from_value(source).unwrap();
    let inputs = comparison::Inputs {
        loaded_modifiers: Some(&loaded),
        ..Default::default()
    };

    assert!(comparison::evaluate(&ledger, &bytes, &inputs).is_err());
}

#[tokio::test]
async fn indistinguishable_scope_types_are_refused() {
    let mut extraction = recorded().await;
    let scopes = &mut extraction.language.scopes.as_mut().unwrap().value;
    let mut twin: ScopeDeclaration = scopes.types[0].clone();
    let id = serde_json::to_value(&twin.id).unwrap();
    twin.id = serde_json::from_value(json!(format!("{}0", id.as_str().unwrap()))).unwrap();
    scopes.types.push(twin);

    let error = snapshot::assemble(&extraction).unwrap_err();

    assert!(error.contains("share the name and keywords"), "{error}");
}

#[tokio::test]
async fn localization_gaps_stay_on_the_link_they_name() {
    let snapshot = snapshot::assemble(&recorded().await).unwrap();
    let gap = gap(
        &snapshot,
        "localization_link:EVENT_TARGET_0#native.localization.unresolved_path",
    )
    .unwrap();

    assert_eq!(
        gap.native_gaps[0].subject.as_deref(),
        Some("EVENT_TARGET_0")
    );
    assert!(
        gap.native_gaps
            .iter()
            .all(|native| native.subject.as_deref() == Some("EVENT_TARGET_0"))
    );
}

#[tokio::test]
async fn loaded_answer_with_other_content_is_refused() {
    let ledger = language_ledger();
    let bytes = snapshot::json_bytes(&snapshot::assemble(&recorded().await).unwrap()).unwrap();
    let mut loaded = loaded().await;
    loaded.value.modifiers.pop();
    let inputs = comparison::Inputs {
        loaded_modifiers: Some(&loaded),
        ..Default::default()
    };

    let error = comparison::evaluate(&ledger, &bytes, &inputs).unwrap_err();

    assert!(
        error.contains("not the one that the snapshot summarizes"),
        "{error}"
    );
}

#[tokio::test]
async fn unreadable_script_docs_logs_are_errors() {
    let ledger = language_ledger();
    let bytes = snapshot::json_bytes(&snapshot::assemble(&recorded().await).unwrap()).unwrap();
    let cases = [
        ("effects.log", ""),
        ("effects.log", "== EFFECT DOCUMENTATION ==\n"),
        (
            "effects.log",
            "== EFFECT DOCUMENTATION ==\nadd_age - Adds age\nSupported Scopes: leader\n\ntruncated - No end\n",
        ),
        ("scopes.log", "== SCOPE DOCUMENTATION ==\nowner - Owner\n"),
        ("localizations.log", "Properties\n GetName\n"),
        ("modifiers.log", "Printing Modifier Definitions:\n"),
    ];

    for (log, text) in cases {
        let inputs = comparison::Inputs {
            script_docs: BTreeMap::from([(log.to_owned(), text.to_owned())]),
            ..Default::default()
        };
        let error = comparison::evaluate(&ledger, &bytes, &inputs).unwrap_err();

        assert!(error.starts_with(log), "{log}: {error}");
    }
}

#[test]
fn define_files_keep_each_path_and_refuse_repeats() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().join("base/00_defines.txt");
    let dlc = root.path().join("dlc/00_defines.txt");

    for path in [&base, &dlc] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "NGameplay = { A = 1 }\n").unwrap();
    }

    let files = pdx_atlas::report::read_define_files(&[base.clone(), dlc]).unwrap();

    assert_eq!(files.len(), 2);
    assert!(pdx_atlas::report::read_define_files(&[base.clone(), base]).is_err());
}

#[tokio::test]
async fn a_closing_separator_ends_the_log_section() {
    let ledger = language_ledger();
    let bytes = snapshot::json_bytes(&snapshot::assemble(&recorded().await).unwrap()).unwrap();
    let inputs = comparison::Inputs {
        script_docs: BTreeMap::from([(
            "effects.log".to_owned(),
            "== EFFECT DOCUMENTATION ==\nadd_age - Adds the age of the scoped leader\nSupported Scopes: leader\n\n\n=================\n".to_owned(),
        )]),
        ..Default::default()
    };
    let report = comparison::evaluate(&ledger, &bytes, &inputs).unwrap();

    assert_eq!(
        list(&report.script_docs[0].lists, "names").agree,
        ["add_age"]
    );
}
