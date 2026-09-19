use super::{Origin, Source, hash};
use pdxscript::{
    Span,
    script::{self, Item, ItemKind, Value},
};

#[derive(Clone)]
struct Key {
    path: Vec<String>,
    span: Span,
}
fn keys(items: &[Item], path: &[String], output: &mut Vec<Key>) {
    for item in items {
        match &item.kind {
            ItemKind::Entry(entry) => {
                let mut path = path.to_vec();
                path.push(entry.key.clone());
                output.push(Key {
                    path: path.clone(),
                    span: item.span.expect("parsed item"),
                });
                if let Value::Container(body) = &entry.value {
                    keys(&body.items, &path, output);
                }
            }
            ItemKind::Container(body) => keys(&body.items, path, output),
            ItemKind::Param { items, .. } => keys(items, path, output),
            _ => {}
        }
    }
}
#[derive(Clone)]
struct Comment {
    line: usize,
    end_line: usize,
    column: usize,
    inline: bool,
    text: String,
}

// Preserve byte positions when exposing commented example assignments to the syntax parser.
fn example_view(source: &str) -> String {
    source
        .split_inclusive('\n')
        .map(|line| {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix('#') else {
                return line.to_string();
            };
            let content = rest.trim_start();
            let key = content.split_whitespace().next().unwrap_or("");
            let assignment = content.split_once('=').is_some_and(|(left, _)| {
                let left = left.trim();
                !left.is_empty()
                    && left
                        .chars()
                        .all(|c| c.is_alphanumeric() || "_@./-$".contains(c))
            });
            if assignment || content.starts_with('}') || key == "{" {
                let offset = line.len() - trimmed.len();
                let mut exposed = line.to_string();
                exposed.replace_range(offset..offset + 1, " ");
                exposed
            } else {
                line.to_string()
            }
        })
        .collect()
}

fn comments(source: &str) -> Vec<Comment> {
    let mut result: Vec<Comment> = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for (index, line) in source.lines().enumerate() {
        let mut start = None;
        for (offset, ch) in line.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if quoted && ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                quoted = !quoted;
            }
            if ch == '#' && !quoted {
                start = Some(offset);
                break;
            }
        }
        let Some(column) = start else { continue };
        let text = line[column..].trim_start_matches('#').trim();
        if text.is_empty() {
            if let Some(previous) = result.last_mut()
                && previous.end_line == index
            {
                previous.text.push('\n');
                previous.end_line = index + 1;
            }
            continue;
        }
        let inline = !line[..column].trim().is_empty();
        if let Some(previous) = result.last_mut()
            && !inline
            && previous.end_line == index
            && (!previous.inline || column >= previous.column)
        {
            previous.text.push('\n');
            previous.text.push_str(text);
            previous.end_line = index + 1;
            continue;
        }
        result.push(Comment {
            line: index + 1,
            end_line: index + 1,
            column,
            inline,
            text: text.into(),
        });
    }
    result
}

/// Attaches preceding, inline, and empty-block comments to parsed game keys.
/// Defines also inherit a leading group comment through adjacent siblings until a blank line.
/// Example files expose commented assignments; parser failures are returned, never hidden.
pub fn parse_comments(file: &str, source: &str) -> (Vec<Source>, Vec<String>) {
    let example = file.rsplit('/').next().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name.contains("example") || name.contains("readme") || name.contains("documentation")
    });
    let view = if example {
        example_view(source)
    } else {
        source.into()
    };
    let document = match script::parse(&view, file) {
        Ok(document) => document,
        Err(error) => return (named_comments(file, source), vec![error.to_string()]),
    };
    let diagnostics = document
        .diagnostics
        .iter()
        .map(|d| format!("{file}:{}: {:?}", d.span.line, d.kind))
        .collect();
    let mut entries = Vec::new();
    keys(&document.items, &[], &mut entries);
    let lines: Vec<_> = view.lines().collect();
    let mut output = named_comments(file, source);
    let sha256 = hash(source.as_bytes());
    for comment in comments(&view) {
        let mut targets: Vec<(&Key, &str)> = Vec::new();
        if comment.inline {
            if let Some(key) = entries
                .iter()
                .filter(|k| k.span.line == comment.line || k.span.end_line == comment.line)
                .max_by_key(|k| {
                    (
                        k.span.end_line == comment.line,
                        if k.span.end_line == comment.line {
                            k.span.end
                        } else {
                            k.span.start
                        },
                    )
                })
            {
                targets.push((key, "inline"));
            }
        } else {
            let next = entries
                .iter()
                .filter(|k| k.span.line == comment.end_line + 1)
                .min_by_key(|k| k.span.start);
            if let Some(next) = next {
                targets.push((next, "leading"));
                if file.starts_with("common/defines/") {
                    for sibling in &entries {
                        if sibling.span.start <= next.span.start
                            || sibling.path.len() != next.path.len()
                            || sibling.path[..sibling.path.len() - 1]
                                != next.path[..next.path.len() - 1]
                        {
                            continue;
                        }
                        let between = &lines[next.span.line..sibling.span.line - 1];
                        if between.iter().any(|line| {
                            line.trim().is_empty() || line.trim_start().starts_with('#')
                        }) {
                            continue;
                        }
                        targets.push((sibling, "group"));
                    }
                }
            }
            // Prose inside a block can document that block, including commented examples.
            if let Some(parent) = entries
                .iter()
                .filter(|k| k.span.line < comment.line && k.span.end_line > comment.end_line)
                .max_by_key(|k| k.path.len())
                && next.is_none()
            {
                targets.push((parent, "block"));
            }
        }
        for (key, association) in targets {
            output.push(Source {
                origin: Origin::ShippedComment,
                file: file.into(),
                sha256: sha256.clone(),
                line: comment.line,
                end_line: comment.end_line,
                key: key.path.clone(),
                association: association.into(),
                text: comment.text.clone(),
            });
        }
    }
    (output, diagnostics)
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_alphanumeric() || "_@.-".contains(c))
}

// Shipped guides also name fields directly ("key -> prose", "key: prose", or commented
// assignments). These are documentation declarations, not active game definitions.
fn named_heading(text: &str) -> Option<(&str, &str)> {
    for delimiter in [" -> ", ":", " - "] {
        if let Some((key, prose)) = text.split_once(delimiter)
            && valid_key(key.trim())
        {
            return Some((key.trim(), prose.trim()));
        }
    }
    let column = comments(text)
        .into_iter()
        .find(|comment| comment.inline)
        .map(|comment| comment.column);
    let (declaration, inline) =
        column.map_or((text, ""), |column| (&text[..column], &text[column..]));
    let key = declaration
        .split_once('=')
        .map_or(declaration, |(key, _)| key)
        .trim();
    if !valid_key(key) {
        return None;
    }
    if !inline.is_empty() {
        return Some((key, inline.trim_start_matches('#').trim()));
    }
    let (_, value) = declaration.split_once('=')?;
    // A few documentation files place prose after a placeholder without a second '#'.
    let prose = value
        .split_once('}')
        .map(|(_, tail)| tail.trim())
        .unwrap_or("");
    Some((key, prose))
}
fn named_comments(file: &str, source: &str) -> Vec<Source> {
    let sha256 = hash(source.as_bytes());
    let mut result = Vec::new();
    for comment in comments(source).into_iter().filter(|c| !c.inline) {
        let mut current: Option<Source> = None;
        for (offset, line) in comment.text.lines().enumerate() {
            let line_number = comment.line + offset;
            let text = line.trim().trim_start_matches('#').trim();
            if let Some((key, prose)) = named_heading(text) {
                if let Some(previous) = current.take()
                    && !previous.text.trim().is_empty()
                {
                    result.push(previous);
                }
                current = Some(Source {
                    origin: Origin::ShippedComment,
                    file: file.into(),
                    sha256: sha256.clone(),
                    line: line_number + usize::from(prose.is_empty()),
                    end_line: line_number,
                    key: vec![key.into()],
                    association: "named_comment".into(),
                    text: prose.into(),
                });
            } else if let Some(previous) = current.as_mut() {
                if text.is_empty() || text.starts_with('}') {
                    if let Some(previous) = current.take()
                        && !previous.text.trim().is_empty()
                    {
                        result.push(previous);
                    }
                } else {
                    if !previous.text.is_empty() {
                        previous.text.push('\n');
                    }
                    previous.text.push_str(text);
                    previous.end_line = line_number;
                }
            }
        }
        if let Some(previous) = current
            && !previous.text.trim().is_empty()
        {
            result.push(previous);
        }
    }
    result
}
