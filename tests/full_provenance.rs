use pdx_atlas::{coverage, ledger, provenance, report};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

fn digest(value: &impl serde::Serialize) -> String {
    format!("{:x}", Sha256::digest(report::json_bytes(value).unwrap()))
}

#[test]
#[ignore = "requires PDX_CONFIG_PATH, PDX_GAME_CONTENT, PDX_ENGINE_DOCS, and ATLAS_REGISTRY_SNAPSHOT"]
fn full_documentation_measurement_matches_pinned_sources_and_reports() {
    let config = std::env::var("PDX_CONFIG_PATH").expect("PDX_CONFIG_PATH is required");
    let game = std::env::var("PDX_GAME_CONTENT").expect("PDX_GAME_CONTENT is required");
    let engine = std::env::var("PDX_ENGINE_DOCS").expect("PDX_ENGINE_DOCS is required");
    let snapshot =
        std::env::var("ATLAS_REGISTRY_SNAPSHOT").expect("ATLAS_REGISTRY_SNAPSHOT is required");
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/documentation-baseline.json")).unwrap();
    let corpus = provenance::read_corpus(Path::new(&game), Path::new(&engine)).unwrap();
    let mut ledger = ledger::inventory(&report::read_sources(Path::new(&config)).unwrap());
    let documentation = provenance::annotate(&mut ledger, &corpus);
    let bytes = std::fs::read(snapshot).unwrap();
    let coverage = coverage::evaluate(&ledger, Some(&bytes)).unwrap();
    let measured = serde_json::to_value(&documentation).unwrap();
    for key in [
        "config_sha256",
        "totals",
        "by_family",
        "source_parse_complete",
        "diagnostics",
    ] {
        assert_eq!(measured[key], expected[key], "{key}");
    }
    assert_eq!(
        documentation.inputs.len(),
        expected["input_files"].as_u64().unwrap() as usize
    );
    assert_eq!(
        digest(&documentation.inputs),
        expected["inputs_sha256"].as_str().unwrap()
    );
    for (file, hash) in [
        ("ledger.json", digest(&ledger)),
        ("coverage.json", digest(&coverage)),
        ("documentation.json", digest(&documentation)),
    ] {
        assert_eq!(
            hash,
            expected["report_sha256"][file].as_str().unwrap(),
            "{file}"
        );
    }
    assert_eq!(coverage.totals.atlas_owned.covered, 0);
    for claim in &ledger.claims {
        assert_eq!(
            claim.property == "documentation",
            claim.provenance.is_some()
        );
        if let Some(attribution) = &claim.provenance {
            assert_eq!(
                attribution.origin == provenance::Origin::Authored,
                documentation.authored.contains(&claim.id)
            );
            for source in &attribution.sources {
                let prefix = match source.origin {
                    provenance::Origin::EngineText => "engine",
                    provenance::Origin::ShippedComment => "content",
                    provenance::Origin::Authored => panic!("authored text has no source"),
                };
                assert_eq!(
                    documentation.inputs[&format!("{prefix}/{}", source.file)],
                    source.sha256
                );
            }
        }
    }
    for example in expected["examples"].as_array().unwrap() {
        let claim = ledger
            .claims
            .iter()
            .find(|c| c.id == example["claim"].as_str().unwrap())
            .unwrap();
        assert_eq!(claim.config_answer, example["text"]);
        let attribution = claim.provenance.as_ref().unwrap();
        assert_eq!(attribution.comparison, provenance::Match::Exact);
        assert_eq!(
            serde_json::to_value(&attribution.sources[0]).unwrap(),
            example["source"]
        );
        assert_eq!(
            claim.config_answer.split_whitespace().collect::<Vec<_>>(),
            attribution.sources[0]
                .text
                .split_whitespace()
                .collect::<Vec<_>>()
        );
    }
}
