use pdx_atlas::report;
use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("ledger") {
        return Err("usage: pdx-atlas ledger --config DIR [--snapshot FILE] [--game-content DIR --engine-docs DIR] --output DIR".into());
    }
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if ![
            "--config",
            "--snapshot",
            "--output",
            "--game-content",
            "--engine-docs",
        ]
        .contains(&key.as_str())
        {
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
    let corpus = match (options.get("--game-content"), options.get("--engine-docs")) {
        (Some(game), Some(engine)) => Some(pdx_atlas::provenance::read_corpus(game, engine)?),
        (None, None) => None,
        _ => return Err("--game-content and --engine-docs must be supplied together".into()),
    };
    let (ledger, coverage) = report::generate(
        config,
        options.get("--snapshot").map(PathBuf::as_path),
        output,
        corpus.as_ref(),
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
    let source_problems = corpus.as_ref().map_or(0, |corpus| corpus.diagnostics.len());
    if source_problems > 0 {
        eprintln!(
            "{source_problems} documentation source parsing diagnostics remain. See documentation.json."
        );
    }
    Ok(ledger.diagnostics.is_empty() && source_problems == 0)
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
