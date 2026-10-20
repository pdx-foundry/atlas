//! The comments of shipped game files, bound to the keys that they document.
#[cfg(test)]
#[path = "shipped_comments/tests.rs"]
mod tests;
use pdxscript::{
    Span,
    script::{self, Item, ItemKind, Value},
};

/// One comment of a shipped file and the key path that it documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Comment {
    /// Path relative to the installation root.
    pub file: String,
    /// First physical line, one-based.
    pub line: usize,
    /// Last physical line, inclusive.
    pub end_line: usize,
    /// Key path that the comment documents.
    pub key: Vec<String>,
    /// Comment text without its markers, one line per source line.
    pub text: String,
}

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
/// Adjacent comment lines that read as one text.
#[derive(Clone)]
struct Run {
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

fn runs(source: &str) -> Vec<Run> {
    let mut result: Vec<Run> = Vec::new();
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
        let prefix = if index == 0 {
            line[..column].trim_start_matches('\u{feff}')
        } else {
            &line[..column]
        };
        let inline = !prefix.trim().is_empty();
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
        result.push(Run {
            line: index + 1,
            end_line: index + 1,
            column,
            inline,
            text: text.into(),
        });
    }
    result
}

/// Binds preceding, inline and empty-block comments to the parsed keys of one shipped file, and
/// reads guide headings such as `key: prose`. Defines also inherit a leading group comment
/// through adjacent siblings until a blank line. Example files expose commented assignments.
/// Returns the comments and the file's parse problems. A file that the parser repairs keeps the
/// comments of the repaired tree; a file that fails to parse keeps only its guide headings.
pub(crate) fn parse(file: &str, source: &str) -> (Vec<Comment>, Vec<String>) {
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
    for comment in runs(&view) {
        let targets = documented_keys(file, &entries, &lines, &comment);

        for key in targets {
            output.push(Comment {
                file: file.into(),
                line: comment.line,
                end_line: comment.end_line,
                key: key.path.clone(),
                text: comment.text.clone(),
            });
        }
    }
    (output, diagnostics)
}

/// The keys that `comment` documents: for an inline comment, the key on its line; otherwise the
/// key on the next line (and, in defines, the siblings of its group), or, with no next key, the
/// innermost block around it.
fn documented_keys<'a>(
    file: &str,
    entries: &'a [Key],
    lines: &[&str],
    comment: &Run,
) -> Vec<&'a Key> {
    if comment.inline {
        return inline_key(entries, comment).into_iter().collect();
    }

    let Some(next) = following_key(entries, comment) else {
        // Prose inside a block can document that block, including commented examples.
        return enclosing_block(entries, comment).into_iter().collect();
    };

    let mut keys = vec![next];

    if file.starts_with("common/defines/") {
        keys.extend(define_group(entries, lines, next));
    }

    keys
}

/// The key that ends on the comment's line, else the last key that starts there.
fn inline_key<'a>(entries: &'a [Key], comment: &Run) -> Option<&'a Key> {
    entries
        .iter()
        .filter(|k| k.span.line == comment.line || k.span.end_line == comment.line)
        .max_by_key(|k| {
            let ends_here = k.span.end_line == comment.line;

            (ends_here, if ends_here { k.span.end } else { k.span.start })
        })
}

/// The first key on the line after the comment.
fn following_key<'a>(entries: &'a [Key], comment: &Run) -> Option<&'a Key> {
    entries
        .iter()
        .filter(|k| k.span.line == comment.end_line + 1)
        .min_by_key(|k| k.span.start)
}

/// The later siblings of a define `first` that share its leading comment: those that follow it
/// with no blank or comment line in between.
fn define_group<'a>(entries: &'a [Key], lines: &[&str], first: &Key) -> Vec<&'a Key> {
    let parent = &first.path[..first.path.len() - 1];

    entries
        .iter()
        .filter(|sibling| {
            let is_later_sibling = sibling.span.start > first.span.start
                && sibling.path.len() == first.path.len()
                && sibling.path[..sibling.path.len() - 1] == *parent;

            if !is_later_sibling {
                return false;
            }

            let between = if first.span.line == sibling.span.line {
                &[][..]
            } else {
                &lines[first.span.line..sibling.span.line - 1]
            };

            !between
                .iter()
                .any(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
        })
        .collect()
}

/// The innermost key whose block contains the comment.
fn enclosing_block<'a>(entries: &'a [Key], comment: &Run) -> Option<&'a Key> {
    entries
        .iter()
        .filter(|k| k.span.line < comment.line && k.span.end_line > comment.end_line)
        .max_by_key(|k| k.path.len())
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
    let column = runs(text)
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
/// The text of `comment` without the guide headings for another key (`other: prose`, or a
/// commented `other = …`) and the lines that continue them until a blank line: those words
/// document the other key, not the comment's own.
pub(crate) fn own_text(comment: &Comment) -> String {
    let own = comment.key.last().map(String::as_str);
    let mut other = false;
    let mut kept = Vec::new();

    for line in comment.text.lines() {
        let text = line.trim().trim_start_matches('#').trim();

        match named_heading(text) {
            Some((key, _)) => other = Some(key) != own,
            None if text.is_empty() || text.starts_with('}') => other = false,
            None => {}
        }

        if !other {
            kept.push(line);
        }
    }

    kept.join("\n")
}

fn named_comments(file: &str, source: &str) -> Vec<Comment> {
    let mut result = Vec::new();
    for comment in runs(source).into_iter().filter(|c| !c.inline) {
        let mut current: Option<Comment> = None;
        for (offset, line) in comment.text.lines().enumerate() {
            let line_number = comment.line + offset;
            let text = line.trim().trim_start_matches('#').trim();
            if let Some((key, prose)) = named_heading(text) {
                if let Some(previous) = current.take()
                    && !previous.text.trim().is_empty()
                {
                    result.push(previous);
                }
                current = Some(Comment {
                    file: file.into(),
                    line: line_number + usize::from(prose.is_empty()),
                    end_line: line_number,
                    key: vec![key.into()],
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
