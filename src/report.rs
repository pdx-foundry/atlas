//! Filesystem boundary for reproducible config inventories and coverage reports.
use crate::{
    coverage::{self, comparison},
    ledger::{self, Ledger, Sources},
};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

fn visit(root: &Path, dir: &Path, sources: &mut Sources) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Symlink input is not supported: {}", path.display()),
            ));
        }
        if kind.is_dir() {
            visit(root, &path, sources)?;
        } else if path.extension().is_some_and(|s| s == "cwt") {
            let relative = path.strip_prefix(root).expect("descendant");
            let name = relative
                .to_str()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Config filenames must be UTF-8",
                    )
                })?
                .replace('\\', "/");
            let source = fs::read_to_string(&path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
            sources.insert(name, source);
        }
    }
    Ok(())
}
/// Writes a comparison report from a rule snapshot, without changing coverage output.
pub fn generate_comparison(
    config: &Path,
    snapshot: &Path,
    output: &Path,
    inputs: &comparison::Inputs,
) -> Result<comparison::Report, Box<dyn std::error::Error>> {
    let ledger = ledger::inventory(&read_sources(config)?);
    let report =
        comparison::evaluate(&ledger, &fs::read(snapshot)?, inputs).map_err(io::Error::other)?;
    fs::create_dir_all(output)?;
    fs::write(output.join("comparison.json"), json_bytes(&report)?)?;
    Ok(report)
}
/// Reads each `script-docs` log of one game version's directory. A missing log is an error.
pub fn read_script_docs(directory: &Path) -> io::Result<BTreeMap<String, String>> {
    comparison::script_docs::LOGS
        .iter()
        .map(|log| {
            let path = directory.join(log);
            let text = fs::read_to_string(&path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;

            Ok(((*log).to_owned(), text))
        })
        .collect()
}

/// Reads define files, keyed by file name.
pub fn read_define_files(paths: &[PathBuf]) -> io::Result<BTreeMap<String, String>> {
    paths
        .iter()
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid define file name")
                })?;
            let text = fs::read_to_string(path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;

            Ok((name.to_owned(), text))
        })
        .collect()
}

/// Reads all CWT files below an explicit root. Rejects symlinks and empty inputs rather than skipping them.
pub fn read_sources(root: &Path) -> io::Result<Sources> {
    if fs::symlink_metadata(root)?.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Config root must not be a symlink",
        ));
    }
    let mut sources = Sources::new();
    visit(root, root, &mut sources)?;
    if sources.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No .cwt files found",
        ));
    }
    Ok(sources)
}
/// Serializes deterministic, pretty JSON with a final newline.
pub fn json_bytes(value: &impl serde::Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
/// Writes the ledger and coverage report.
pub fn generate(
    config: &Path,
    snapshot: Option<&Path>,
    output: &Path,
) -> Result<(Ledger, coverage::Report), Box<dyn std::error::Error>> {
    let sources = read_sources(config)?;
    let ledger = ledger::inventory(&sources);
    let input = snapshot.map(fs::read).transpose()?;
    let report = coverage::evaluate(&ledger, input.as_deref()).map_err(io::Error::other)?;
    let ledger_bytes = json_bytes(&ledger)?;
    let report_bytes = json_bytes(&report)?;
    fs::create_dir_all(output)?;
    fs::write(output.join("ledger.json"), ledger_bytes)?;
    fs::write(output.join("coverage.json"), report_bytes)?;
    Ok((ledger, report))
}
