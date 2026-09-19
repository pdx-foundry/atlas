use pdx_atlas::report;
use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("ledger") {
        return Err("usage: pdx-atlas ledger --config DIR [--snapshot FILE] --output DIR".into());
    }
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if !["--config", "--snapshot", "--output"].contains(&key.as_str()) {
            return Err(format!("Unknown argument: {key}").into());
        }
        let value = args.next().ok_or("Missing argument value")?;
        if value.starts_with("--") {
            return Err(format!("Missing value for {key}").into());
        }
        if options.insert(key.clone(), PathBuf::from(value)).is_some() {
            return Err(format!("Duplicate argument: {key}").into());
        }
    }
    let config = options.get("--config").ok_or("--config is required")?;
    let output = options.get("--output").ok_or("--output is required")?;
    let (ledger, coverage) = report::generate(
        config,
        options.get("--snapshot").map(PathBuf::as_path),
        output,
    )?;
    let headline = &coverage.totals.atlas_owned;
    let percentage = headline
        .percent
        .map_or("not applicable".into(), |n| format!("{n:.6}%"));
    println!(
        "{} CWT files; {} claims; Atlas-owned coverage: {}/{} ({percentage})",
        ledger.files.len(),
        ledger.claims.len(),
        headline.covered,
        headline.total
    );
    println!("Reports: {}", output.display());
    if !ledger.diagnostics.is_empty() {
        eprintln!(
            "{} source diagnostics remain; percentages cover inventoried claims only. See ledger.json.",
            ledger.diagnostics.len()
        );
    }
    Ok(ledger.diagnostics.is_empty())
}
fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
