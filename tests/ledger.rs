use pdx_atlas::{ledger::*, report::json_bytes};
fn scan(text: &str) -> Ledger {
    inventory(&[("common/example.cwt".into(), text.into())].into())
}
#[test]
fn every_meaningful_line_is_accounted_for() {
    let source = "types = {\n type[example] = { path = game/common/example }\n}\n# ignored\nexample = {\n ## cardinality = 0..1\n ### Documentation\n #### continuation\n field = <other>\n field = bool\n}\n";
    let ledger = scan(source);
    assert!(ledger.diagnostics.is_empty());
    assert_eq!(ledger.files[0].lines.len(), 10);
    assert!(ledger.files[0].lines.iter().all(|l| !l.claims.is_empty()));
    assert!(
        ledger
            .claims
            .iter()
            .filter(|c| c.owner == Owner::EngineFact)
            .all(|c| c.expected_method.is_some())
    );
    let docs: Vec<_> = ledger
        .claims
        .iter()
        .filter(|c| c.property == "documentation")
        .collect();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].config_answer, "Documentation\ncontinuation");
    assert!(docs[0].provisional_owner);
    let values: Vec<_> = ledger
        .claims
        .iter()
        .filter(|c| c.property == "value_form" && c.subject.last().unwrap() == "field")
        .collect();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].question, values[1].question);
    assert_ne!(values[0].id, values[1].id);
}
#[test]
fn answers_and_whitespace_do_not_define_question_identity() {
    let a = scan("x = bool\n");
    let b = scan("\n # moved\n x = int\n");
    assert_eq!(
        a.claims.iter().map(|c| &c.id).collect::<Vec<_>>(),
        b.claims.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    assert_ne!(a.config_sha256, b.config_sha256);
}
#[test]
fn unknown_syntax_and_annotations_are_not_hidden() {
    for text in [
        "x = {\n z = bool",
        "## unknown = yes\nx = bool",
        "=",
        "## orphan",
        "x = \"broken",
    ] {
        let ledger = scan(text);
        assert!(!ledger.diagnostics.is_empty(), "{text}");
        assert!(
            ledger.files[0]
                .lines
                .iter()
                .all(|l| !l.claims.is_empty() || !l.diagnostics.is_empty())
        );
    }
}
#[test]
fn owner_and_method_classification_separates_policy() {
    let ledger = scan(
        "types = { type[x] = { subtype[foo] = { flag = yes } } }\nx = {\n ## severity = warning\n ## cardinality = ~1..1\n field = int\n}",
    );
    for property in [
        "subtype_partition",
        "consumer_annotation",
        "soft_cardinality_minimum",
    ] {
        assert!(
            ledger
                .claims
                .iter()
                .filter(|c| c.property == property)
                .all(|c| c.owner == Owner::ConsumerPolicy)
        );
        assert!(ledger.claims.iter().any(|c| c.property == property));
    }
    assert!(
        ledger
            .claims
            .iter()
            .any(|c| c.property == "conditional_constraint"
                && c.expected_method.as_ref().unwrap().ticket == "SDK-541")
    );
    let schema = ledger
        .claims
        .iter()
        .find(|c| c.property == "value_form" && c.config_answer == "int")
        .unwrap();
    assert_eq!(schema.expected_method.as_ref().unwrap().ticket, "SDK-544");
}
#[test]
fn source_accounting_handles_multiline_quotes_and_unterminated_last_line() {
    let ledger = scan("x = \"first\n# still text\nlast\"");
    assert!(ledger.diagnostics.is_empty());
    assert_eq!(ledger.files[0].physical_lines, 3);
    assert_eq!(ledger.files[0].lines.len(), 3);
}
#[test]
fn deterministic_input_order() {
    let a = inventory(
        &[
            ("b.cwt".into(), "b = yes".into()),
            ("a.cwt".into(), "a = no".into()),
        ]
        .into(),
    );
    let b = inventory(
        &[
            ("a.cwt".into(), "a = no".into()),
            ("b.cwt".into(), "b = yes".into()),
        ]
        .into(),
    );
    assert_eq!(json_bytes(&a).unwrap(), json_bytes(&b).unwrap());
}

#[test]
fn malformed_known_annotations_are_reported() {
    for annotation in [
        "cardinality = nonsense",
        "cardinality 0..1",
        "scopes =",
        "replace_scopes = { this = country",
    ] {
        let ledger = scan(&format!("## {annotation}\nx = bool"));
        assert!(
            ledger
                .diagnostics
                .iter()
                .any(|d| d.kind == "malformed-annotation"),
            "{annotation}"
        );
    }
}

#[test]
fn mixed_cardinality_and_scope_operations_are_independent_questions() {
    let ledger = scan(
        "## cardinality = ~1..1\n## push_scope = country\n## replace_scope = planet\nx = bool",
    );
    let minimum = ledger
        .claims
        .iter()
        .find(|c| c.property == "soft_cardinality_minimum")
        .unwrap();
    let maximum = ledger
        .claims
        .iter()
        .find(|c| c.property == "cardinality_maximum")
        .unwrap();
    assert_eq!(minimum.owner, Owner::ConsumerPolicy);
    assert_eq!(maximum.owner, Owner::EngineFact);
    assert_eq!(maximum.expected_method.as_ref().unwrap().ticket, "SDK-541");
    let scopes: Vec<_> = ledger
        .claims
        .iter()
        .filter(|c| c.property == "scope_context")
        .collect();
    assert_eq!(scopes.len(), 2);
    assert_ne!(scopes[0].question, scopes[1].question);
}
