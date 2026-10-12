use pdx_atlas::{
    coverage::{self, registries::Join},
    ledger,
};
use pdx_native::{Completeness, Gap, GapKind, Native, Registry};
use std::path::Path;

const TYPES: &str = r#"
types = {
    type[tradition] = { path = "game/common/traditions" }
    type[swapped_tradition] = { path = "game/common/traditions" }
    type[relic] = { path = "game/common/relics" }
    type[precursor] = { path = "common/precursor_civilizations" }
    type[strategic_resource] = { path = "game/common/strategic_resources" }
    type[alert] = {
        path = "game/common"
        path_file = "alerts.txt"
    }
    type[sprite] = { path = "game/gfx" }
    type[resource_group] = { path = "game/interface/resource_groups" }
}
tradition = { cost = int }
relic = { score = int }
"#;

fn ledger() -> ledger::Ledger {
    ledger::inventory(&[("common/types.cwt".into(), TYPES.into())].into())
}

#[test]
fn each_config_type_has_one_join_to_an_authored_registry_list() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452");
    let mut registries = Native::from_recorded_answers(path)
        .unwrap()
        .registries()
        .unwrap();
    registries.value = [
        "common/traditions",
        "common/relics",
        "common/precursor_civilizations",
        "common/lawsuits",
        "interface/resource_groups",
    ]
    .map(|name| Registry { name: name.into() })
    .into();
    registries.gaps.push(Gap {
        kind: GapKind::UnnamedRegistries,
        subject: None,
        detail: "2 registries were found without one content directory.".into(),
    });
    registries.completeness = Completeness::Partial;

    let report = coverage::registries::compare(&ledger(), &registries);

    let join = |type_name: &str| {
        let row = report
            .rows
            .iter()
            .find(|row| row.type_name == format!("type[{type_name}]"))
            .unwrap();
        (row.directory.as_str(), row.join, row.gate)
    };
    assert_eq!(join("relic"), ("common/relics", Join::Named, true));
    assert_eq!(
        join("precursor"),
        ("common/precursor_civilizations", Join::Named, true)
    );
    assert_eq!(
        join("tradition"),
        ("common/traditions", Join::SeveralTypesInDirectory, true)
    );
    assert_eq!(
        join("strategic_resource"),
        ("common/strategic_resources", Join::NoNamedRegistry, true)
    );
    assert_eq!(join("alert"), ("common", Join::NoNamedRegistry, true));
    assert_eq!(join("sprite"), ("gfx", Join::DeferredFormatFamily, false));
    assert_eq!(
        join("resource_group"),
        ("interface/resource_groups", Join::Named, false)
    );
    assert_eq!(report.rows.len(), 8);
    assert_eq!(report.gate_types, 6);
    assert_eq!(report.directories_in_both, 4);
    assert_eq!(report.engine_only, ["common/lawsuits"]);
    assert_eq!(report.native_completeness, Completeness::Partial);
    assert!(report.inventory_complete);
    assert!(
        report
            .native_gaps
            .iter()
            .any(|gap| gap.kind == GapKind::UnnamedRegistries)
    );
    let reasons = serde_json::to_value(&report.types_by_join).unwrap();
    assert_eq!(
        reasons,
        serde_json::json!({
            "named": 3,
            "several_types_in_directory": 2,
            "no_named_registry": 2,
            "deferred_format_family": 1,
        })
    );
}

#[test]
fn per_type_totals_add_up_to_per_file_totals() {
    let report = coverage::evaluate(&ledger(), None).unwrap();

    let file = &report.by_type["common/types.cwt"];
    let tradition = &file.types["type[tradition]"];
    assert!(tradition.all_claims.total > 1);
    assert_eq!(tradition.all_claims.covered, 0);
    assert!(file.types["type[sprite]"].all_claims.total > 0);
    assert!(file.outside_types.all_claims.total > 0);
    for (path, totals) in &report.by_file {
        let types = &report.by_type[path];
        let sum: usize = types.types.values().map(|t| t.all_claims.total).sum();
        assert_eq!(
            sum + types.outside_types.all_claims.total,
            totals.all_claims.total
        );
    }
}
