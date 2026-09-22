use pdx_atlas::{extraction, report, snapshot};
use pdx_native::{GameOptions, Native, supervisor};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, ErrorKind},
    path::PathBuf,
    process::{Command, ExitCode},
};

fn run_ledger() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    args.next();
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

fn run_compare() -> Result<bool, Box<dyn std::error::Error>> {
    let args = std::env::args().skip(2).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: pdx-atlas compare CONFIG_DIR SNAPSHOT_FILE OUTPUT_DIR".into());
    }
    let output = PathBuf::from(&args[2]);
    let report =
        report::generate_comparison(&PathBuf::from(&args[0]), &PathBuf::from(&args[1]), &output)?;
    println!(
        "{} comparison entries; report: {}",
        report.entries.len(),
        output.join("comparison.json").display()
    );
    Ok(report.inventory_complete)
}

enum SnapshotMode {
    Live {
        installation: PathBuf,
        answers: PathBuf,
        seconds: Option<u64>,
    },
    Recorded {
        answers: PathBuf,
    },
}

fn parse_snapshot_args(
    args: &[OsString],
) -> Result<(SnapshotMode, PathBuf), Box<dyn std::error::Error>> {
    if args.first().is_some_and(|arg| arg == "--recorded") {
        if args.len() != 3 {
            return Err("usage: pdx-atlas snapshot --recorded ANSWERS OUTPUT".into());
        }
        return Ok((
            SnapshotMode::Recorded {
                answers: PathBuf::from(&args[1]),
            },
            PathBuf::from(&args[2]),
        ));
    }
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: pdx-atlas snapshot INSTALLATION ANSWERS OUTPUT [STARTUP_SECONDS]".into(),
        );
    }
    let seconds = match args.get(3) {
        Some(value) => Some(value.to_str().ok_or("Invalid startup seconds")?.parse()?),
        None => None,
    };
    Ok((
        SnapshotMode::Live {
            installation: PathBuf::from(&args[0]),
            answers: PathBuf::from(&args[1]),
            seconds,
        },
        PathBuf::from(&args[2]),
    ))
}

fn run_snapshot() -> Result<bool, Box<dyn std::error::Error>> {
    let (mode, output) = parse_snapshot_args(&std::env::args_os().skip(2).collect::<Vec<_>>())?;
    let mut checksum_path = output.clone().into_os_string();
    checksum_path.push(".sha256");
    let checksum_path = PathBuf::from(checksum_path);
    for path in [&checksum_path, &output] {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let (native, seconds) = match mode {
        SnapshotMode::Live {
            installation,
            answers,
            seconds,
        } => (
            Native::open(installation)?.record_answers_to(answers),
            seconds,
        ),
        SnapshotMode::Recorded { answers } => (Native::from_recorded_answers(answers)?, None),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let executable = std::env::current_exe()?;
    let extraction = runtime.block_on(extraction::collect(&native, || {
        let mut host = Command::new(&executable);
        host.arg("--supervisor");
        let mut options = GameOptions::new(host);
        if let Some(seconds) = seconds {
            options.startup_seconds = seconds;
        }
        options
    }));
    let snapshot = snapshot::assemble(&extraction)?;
    let bytes = snapshot::json_bytes(&snapshot)?;
    std::fs::write(&output, &bytes)?;
    std::fs::write(checksum_path, format!("{:x}\n", Sha256::digest(&bytes)))?;
    println!(
        "{} rules; {} gaps",
        snapshot.rules.len(),
        snapshot.gaps.len()
    );
    Ok(extraction.complete())
}

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("--supervisor") => supervisor::serve(io::stdin(), io::stdout())
            .map(|()| true)
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>),
        Some("ledger") => run_ledger(),
        Some("compare") => run_compare(),
        Some("snapshot") => run_snapshot(),
        _ => Err("usage: pdx-atlas ledger --config DIR [--snapshot FILE] --output DIR | compare CONFIG_DIR SNAPSHOT_FILE OUTPUT_DIR | snapshot INSTALLATION ANSWERS OUTPUT [STARTUP_SECONDS] | snapshot --recorded ANSWERS OUTPUT".into()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
