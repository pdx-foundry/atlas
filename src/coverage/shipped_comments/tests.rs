use super::parse;

#[test]
fn comments_bind_to_keys_without_crossing_groups_or_reading_quoted_hashes() {
    let text = "\u{feff}N = {\r\n # shared documentation\r\n A = 1\r\n B = 2 # B only\r\n\r\n C = \"not # a comment\"\r\n # next entry\r\n D = { x = 1 } # whole block\r\n E = { # opening block\r\n  y = 2\r\n }\r\n}\r\n";
    let (parsed, errors) = parse("common/defines/00.txt", text);

    assert!(errors.is_empty(), "{errors:?}");

    for key in ["A", "B"] {
        assert!(
            parsed
                .iter()
                .any(|s| s.key == ["N", key] && s.text == "shared documentation")
        );
    }

    assert!(!parsed.iter().any(|s| s.key == ["N", "C"]));
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["N", "B"] && s.text == "B only" && s.line == 4)
    );
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["N", "D"] && s.text == "whole block")
    );
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["N", "E"] && s.text == "opening block")
    );
    assert!(!parsed.iter().any(|s| s.text.contains("not #")));
}

#[test]
fn documentation_guides_and_empty_example_blocks_keep_locations() {
    let text = "# hostility -> Will this empire attack its neighbors?\n# field = yes # The first line\n#             # and the second line\n\n# example = {\n# condition = {\n#   A trigger for the condition.\n# }\n# }\n";
    let (parsed, _) = parse("common/things/00_example.txt", text);

    assert!(parsed.iter().any(|s| s.key == ["hostility"]
        && s.text == "Will this empire attack its neighbors?"
        && s.line == 1));
    assert!(parsed.iter().any(|s| s.key == ["field"]
        && s.text == "The first line\nand the second line"
        && s.line == 2
        && s.end_line == 3));
    assert!(parsed.iter().any(|s| s.key == ["example", "condition"]
        && s.text == "A trigger for the condition."
        && s.line == 7));
}

#[test]
fn parser_failures_are_reported_without_discarding_guide_headings() {
    let (parsed, errors) = parse(
        "common/things/broken.txt",
        "# field: source text\nbroken = \"unterminated",
    );

    assert_eq!(errors.len(), 1);
    assert!(
        parsed
            .iter()
            .any(|s| s.key == ["field"] && s.text == "source text")
    );
}

#[test]
fn commented_quoted_hashes_are_not_documentation_delimiters() {
    let (parsed, errors) = parse(
        "common/things/guide.txt",
        "# field = \"not # prose\" # Actual documentation\n",
    );

    assert!(errors.is_empty());
    assert!(
        parsed
            .iter()
            .any(|source| source.key == ["field"] && source.text == "Actual documentation")
    );
    assert!(!parsed.iter().any(|source| source.text.contains("not #")));
}

#[test]
fn same_line_define_siblings_share_a_leading_group() {
    let (parsed, errors) = parse(
        "common/defines/00.txt",
        "N = {\n # Shared group\n A = 1 B = 2\n}\n",
    );

    assert!(errors.is_empty());

    for key in ["A", "B"] {
        assert!(
            parsed
                .iter()
                .any(|source| source.key == ["N", key] && source.text == "Shared group")
        );
    }
}

#[test]
fn leading_bom_does_not_turn_first_line_documentation_into_inline_text() {
    let (parsed, errors) = parse(
        "common/things/00.txt",
        "\u{feff}# Leading documentation\nfield = 1\n",
    );

    assert!(errors.is_empty());
    assert!(parsed.iter().any(|source| source.key == ["field"]
        && source.text == "Leading documentation"
        && source.line == 1));

    let (parsed, errors) = parse(
        "common/things/guide.txt",
        "\u{feff}# field: Named documentation\n",
    );

    assert!(errors.is_empty());
    assert!(parsed.iter().any(|source| source.key == ["field"]
        && source.text == "Named documentation"
        && source.line == 1));
}
