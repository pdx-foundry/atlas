use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn recording() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m45")
}

fn run(recording: &Path, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdx-atlas"))
        .args(["snapshot", "--recorded"])
        .arg(recording)
        .arg(output)
        .output()
        .unwrap()
}

#[test]
fn recorded_command_writes_identical_bytes_and_matching_digest() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first.json");
    let second = root.path().join("second.json");
    for output in [&first, &second] {
        let result = run(&recording(), output);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = fs::read(output).unwrap();
        let digest = fs::read_to_string(format!("{}.sha256", output.display())).unwrap();
        assert_eq!(digest, format!("{:x}\n", Sha256::digest(&bytes)));
    }
    assert_eq!(fs::read(first).unwrap(), fs::read(second).unwrap());
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn missing_recorded_answer_writes_an_explicit_gap_and_exits_nonzero() {
    let root = tempfile::tempdir().unwrap();
    let answers = root.path().join("answers");
    copy_tree(&recording(), &answers);
    fs::remove_file(answers.join("registry_fields/common/traditions.json")).unwrap();
    let output = root.path().join("snapshot.json");
    let result = run(&answers, &output);
    assert_eq!(result.status.code(), Some(2));
    let snapshot: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert!(
        snapshot["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["id"] == "registry:common/traditions#fields")
    );
}

#[test]
fn failed_registry_discovery_publishes_no_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let answers = root.path().join("answers");
    copy_tree(&recording(), &answers);
    fs::write(
        answers.join("registries.json"),
        br#"{"Err":{"Method":"review: registry discovery failed"}}"#,
    )
    .unwrap();
    let output = root.path().join("snapshot.json");
    let result = run(&answers, &output);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Native registry discovery failed"));
    assert!(!output.exists());
    assert!(!root.path().join("snapshot.json.sha256").exists());
}

#[test]
fn partial_unrelated_registry_listing_keeps_only_applicable_subjects() {
    let root = tempfile::tempdir().unwrap();
    let answers = root.path().join("answers");
    fs::create_dir_all(answers.join("registry_fields/common")).unwrap();
    for file in [
        "build.json",
        "registries.json",
        "registry_fields/common/relics.json",
    ] {
        fs::copy(recording().join(file), answers.join(file)).unwrap();
    }
    let path = answers.join("registries.json");
    let mut registries: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    registries["Ok"]["completeness"] = "Partial".into();
    registries["Ok"]["value"] = serde_json::json!([{"name":"common/relics"}]);
    fs::write(&path, serde_json::to_vec(&registries).unwrap()).unwrap();
    let output = root.path().join("snapshot.json");
    let result = run(&answers, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let snapshot: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert_eq!(
        snapshot["coverage"]["registries"],
        serde_json::json!(["common/relics"])
    );
    assert!(
        snapshot["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|subject| { subject["registry"] == "common/relics" })
    );
    assert!(snapshot["gaps"].as_array().unwrap().iter().all(|gap| {
        !gap["subject"]
            .as_str()
            .unwrap()
            .contains("common/tradition")
    }));
}
