use super::{Origin, Source, hash};

/// Reads description and usage lines from a script-docs effects or triggers dump.
/// Scope metadata is excluded, and every returned entry retains physical source lines.
pub fn parse_engine_docs(file: &str, text: &str) -> Vec<Source> {
    let mut result = Vec::new();
    let mut current: Option<Source> = None;
    let sha256 = hash(text.as_bytes());
    for (index, line) in text.lines().enumerate() {
        if let Some((name, description)) = line.split_once(" - ")
            && !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            if let Some(source) = current.take() {
                result.push(source);
            }
            current = Some(Source {
                origin: Origin::EngineText,
                file: file.into(),
                sha256: sha256.clone(),
                line: index + 1,
                end_line: index + 1,
                key: vec![name.into()],
                association: "engine".into(),
                text: description.into(),
            });
        } else if line.starts_with("Supported Scopes:")
            || line.starts_with("Supported Targets:")
            || line.starts_with("==")
        {
            if let Some(source) = current.take() {
                result.push(source);
            }
        } else if let Some(source) = current.as_mut() {
            source.text.push('\n');
            source.text.push_str(line);
            source.end_line = index + 1;
        }
    }
    if let Some(source) = current {
        result.push(source);
    }
    result
}
