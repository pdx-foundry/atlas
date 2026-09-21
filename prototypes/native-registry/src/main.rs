//! Dedicated supervisor role and static, live, or recorded question commands.
use atlas_native_consumer::{collect, describe, frozen};
use pdx_native::{Disposal, GameOptions, Native, supervisor};
use std::{io, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.as_slice() == ["--supervisor"] {
        supervisor::serve(io::stdin(), io::stdout())?;
        return Ok(());
    }
    let command = arguments.first().and_then(|arg| arg.to_str());
    match command {
        Some("describe") if arguments.len() == 2 => {
            let native = Native::open(std::path::PathBuf::from(&arguments[1]))?;
            let answer = describe(&native);
            serde_json::to_writer_pretty(io::stdout(), &answer)?;
            answer?;
        }
        Some("live") if (2..=4).contains(&arguments.len()) => {
            let native = Native::open(std::path::PathBuf::from(&arguments[1]))?;
            let native = match arguments.get(2) {
                Some(directory) => native.record_answers_to(std::path::PathBuf::from(directory)),
                None => native,
            };
            run(&native, arguments.get(3))?;
        }
        Some("recorded") if arguments.len() == 2 => {
            let native = Native::from_recorded_answers(std::path::PathBuf::from(&arguments[1]))?;
            run(&native, None)?;
        }
        Some("frozen") if (3..=4).contains(&arguments.len()) => {
            let native = Native::open(std::path::PathBuf::from(&arguments[1]))?
                .record_answers_to(std::path::PathBuf::from(&arguments[2]));
            run_frozen(&native, arguments.get(3))?;
        }
        Some("frozen-recorded") if arguments.len() == 2 => {
            let native = Native::from_recorded_answers(std::path::PathBuf::from(&arguments[1]))?;
            run_frozen(&native, None)?;
        }
        _ => return Err("usage: atlas-native-consumer describe INSTALLATION | live INSTALLATION [ANSWERS [STARTUP_SECONDS]] | recorded ANSWERS | frozen INSTALLATION ANSWERS [STARTUP_SECONDS] | frozen-recorded ANSWERS".into()),
    }
    Ok(())
}

fn run_frozen(
    native: &Native,
    seconds: Option<&std::ffi::OsString>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seconds: Option<u64> = match seconds {
        Some(value) => Some(value.to_str().ok_or("Invalid startup seconds")?.parse()?),
        None => None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let executable = std::env::current_exe()?;
    let report = runtime.block_on(frozen::run(native, || {
        let mut host = Command::new(&executable);
        host.arg("--supervisor");
        let mut options = GameOptions::new(host);
        if let Some(seconds) = seconds {
            options.startup_seconds = seconds;
        }
        options
    }));
    serde_json::to_writer_pretty(io::stdout(), &report)?;
    if report.summary.unanswered != 0
        || report.answers.sessions.iter().any(|session| {
            !matches!(
                session.termination,
                Ok(Disposal::Confirmed | Disposal::NotApplicable)
            )
        })
    {
        return Err("Incomplete frozen flow; answers and errors are on stdout".into());
    }
    Ok(())
}

fn run(
    native: &Native,
    seconds: Option<&std::ffi::OsString>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut host = Command::new(std::env::current_exe()?);
    host.arg("--supervisor");
    let mut options = GameOptions::new(host);
    if let Some(seconds) = seconds {
        options.startup_seconds = seconds.to_str().ok_or("Invalid startup seconds")?.parse()?;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let observations = runtime.block_on(collect(native, options));
    // Serialize only after close, including when a question failed.
    serde_json::to_writer_pretty(io::stdout(), &observations)?;
    if !matches!(
        observations.termination,
        Ok(Disposal::Confirmed | Disposal::NotApplicable)
    ) || observations.queries.values().any(Result::is_err)
    {
        return Err("Incomplete attempt; answers and errors are on stdout".into());
    }
    Ok(())
}
