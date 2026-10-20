use std::{fs, process::Command};

#[test]
fn compare_command_writes_separate_report_from_recorded_rule_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("traditions.cwt"),
        "types = { type[tradition] = { path = \"game/common/traditions\" } }\ntradition = { unlocks_agenda = int }\n",
    )
    .unwrap();
    let snapshot = root.path().join("snapshot.json");
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452");
    let produced = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .args(["snapshot", "--recorded"])
        .arg(fixture)
        .arg(&snapshot)
        .output()
        .unwrap();
    assert!(produced.status.success(), "{produced:?}");
    let output = root.path().join("comparison");
    let compared = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .arg("compare")
        .arg(&config)
        .arg(&snapshot)
        .arg(&output)
        .output()
        .unwrap();
    assert!(compared.status.success(), "{compared:?}");
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("comparison.json")).unwrap()).unwrap();
    assert_eq!(report["format_version"], 3);
    assert!(report["entry_scope_checks"].is_object());
    assert!(
        report["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["status"] == "same")
    );
    assert!(!output.join("coverage.json").exists());
}
#[test]
fn relocated_inputs_produce_identical_bytes_and_diagnostics_fail_visibly() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["first", "second"] {
        fs::create_dir(root.path().join(dir)).unwrap();
        fs::write(root.path().join(dir).join("rules.cwt"), "x = bool\n").unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
            .args(["ledger", "--config"])
            .arg(root.path().join(dir))
            .arg("--output")
            .arg(root.path().join(format!("{dir}-out")))
            .arg("--dashboard")
            .arg(root.path().join("dashboard"))
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
    }
    for name in ["covered/index.txt", "dashboard.html"] {
        assert!(root.path().join("first-out").join(name).is_file());
    }
    assert_eq!(
        fs::read(root.path().join("second-out/dashboard.html")).unwrap(),
        fs::read(root.path().join("dashboard/dashboard.html")).unwrap()
    );
    for name in ["ledger.json", "coverage.json"] {
        assert_eq!(
            fs::read(root.path().join("first-out").join(name)).unwrap(),
            fs::read(root.path().join("second-out").join(name)).unwrap()
        );
    }
    fs::write(root.path().join("first/rules.cwt"), "x = {").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .args(["ledger", "--config"])
        .arg(root.path().join("first"))
        .arg("--output")
        .arg(root.path().join("broken-out"))
        .arg("--dashboard")
        .arg(root.path().join("dashboard"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(root.path().join("broken-out/ledger.json").is_file());
}

#[cfg(unix)]
#[test]
fn symlinked_config_root_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("config")).unwrap();
    fs::write(root.path().join("config/rules.cwt"), "x = bool").unwrap();
    std::os::unix::fs::symlink(root.path().join("config"), root.path().join("link")).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .args(["ledger", "--config"])
        .arg(root.path().join("link"))
        .arg("--output")
        .arg(root.path().join("out"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("symlink")
    );
}

#[test]
fn a_dashboard_folder_equal_to_the_output_keeps_the_page() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("config")).unwrap();
    fs::write(root.path().join("config/rules.cwt"), "x = bool\n").unwrap();
    let output = root.path().join("out");
    let result = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .args(["ledger", "--config"])
        .arg(root.path().join("config"))
        .arg("--output")
        .arg(&output)
        .arg("--dashboard")
        .arg(&output)
        .output()
        .unwrap();

    assert!(result.status.success(), "{result:?}");
    assert!(
        fs::read_to_string(output.join("dashboard.html"))
            .unwrap()
            .ends_with("</html>\n")
    );
}

#[test]
fn a_run_without_a_live_observation_compares_only_with_a_baseline_of_its_kind() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("traditions.cwt"),
        "types = { type[tradition] = { path = \"game/common/traditions\" } }\ntradition = { unlocks_agenda = int }\n",
    )
    .unwrap();
    let snapshot = root.path().join("snapshot.json");
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452");
    let produced = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .args(["snapshot", "--recorded"])
        .arg(fixture)
        .arg(&snapshot)
        .output()
        .unwrap();
    assert!(produced.status.success(), "{produced:?}");
    let ledger = |output: &str, baseline: Option<&std::path::Path>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"));
        command
            .args(["ledger", "--config"])
            .arg(&config)
            .arg("--snapshot")
            .arg(&snapshot)
            .arg("--output")
            .arg(root.path().join(output))
            .arg("--dashboard")
            .arg(root.path().join("dashboard"));
        if let Some(baseline) = baseline {
            command.arg("--baseline").arg(baseline);
        }
        command.output().unwrap()
    };
    let page =
        |output: &str| fs::read_to_string(root.path().join(output).join("dashboard.html")).unwrap();

    let first = ledger("first", None);

    assert!(first.status.success(), "{first:?}");
    assert!(page("first").contains("\"baseline\":null"));

    let first_baseline = root.path().join("first/covered");
    let second = ledger("second", Some(&first_baseline));

    assert!(second.status.success(), "{second:?}");
    assert!(!page("second").contains("\"baseline\":null"));

    let index = first_baseline.join("index.txt");
    let text = fs::read_to_string(&index).unwrap();
    assert!(text.contains("observed_live false"));
    fs::write(
        &index,
        text.replace("observed_live false", "observed_live true"),
    )
    .unwrap();
    let live_baseline = ledger("third", Some(&first_baseline));

    assert_eq!(live_baseline.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&live_baseline.stderr).contains("do not compare"));
}
