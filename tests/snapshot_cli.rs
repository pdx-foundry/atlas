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
