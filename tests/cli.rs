use std::{fs, process::Command};
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
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
    }
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
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(root.path().join("broken-out/ledger.json").is_file());
}
