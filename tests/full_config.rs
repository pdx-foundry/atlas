use pdx_atlas::{coverage, ledger, report};
use serde_json::Value;
use sha2::{Digest, Sha256};
#[test]
#[ignore = "requires PDX_CONFIG_PATH and ATLAS_REGISTRY_SNAPSHOT; fails when either is missing"]
fn pinned_config_and_current_registry_snapshot() {
    let config = std::env::var("PDX_CONFIG_PATH").expect("PDX_CONFIG_PATH is required");
    let snapshot =
        std::env::var("ATLAS_REGISTRY_SNAPSHOT").expect("ATLAS_REGISTRY_SNAPSHOT is required");
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/full-config-baseline.json")).unwrap();
    let sources = report::read_sources(std::path::Path::new(&config)).unwrap();
    let ledger = ledger::inventory(&sources);
    let bytes = std::fs::read(snapshot).unwrap();
    let coverage = coverage::evaluate(&ledger, Some(&bytes)).unwrap();
    assert_eq!(
        ledger.config_sha256,
        expected["config_sha256"].as_str().unwrap()
    );
    assert_eq!(
        coverage.snapshot_sha256.as_deref(),
        expected["snapshot_sha256"].as_str()
    );
    assert_eq!(ledger.files.len(), 172);
    assert!(
        ledger
            .files
            .iter()
            .flat_map(|f| &f.lines)
            .all(|l| !l.claims.is_empty() || !l.diagnostics.is_empty())
    );
    assert!(
        ledger
            .claims
            .iter()
            .filter(|c| c.owner == ledger::Owner::EngineFact)
            .all(|c| c.expected_method.is_some())
    );
    assert_eq!(coverage.totals.atlas_owned.covered, 0);
    assert_eq!(
        format!("{:x}", Sha256::digest(report::json_bytes(&ledger).unwrap())),
        expected["ledger_sha256"].as_str().unwrap()
    );
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(report::json_bytes(&coverage).unwrap())
        ),
        expected["coverage_sha256"].as_str().unwrap()
    );
}
