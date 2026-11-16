//! The `# unref: ignore[...]` comments, which ignore the findings of their line.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::models::{Finding, Rule};
use crate::positions::{SourcePositions, to_u32};

const MARKER: &str = "unref:";
/// The start of an ignore comment, which is the start of a comment, or follows another comment on its
/// line.
static START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\s*unref:\s*ignore").expect("valid pattern"));

const NO_CODES: &str = "expected the codes to ignore, in brackets";

/// A code that an ignore comment names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IgnoredCode {
    /// The code, which may not be known.
    pub code: String,
    /// The column of the code, starting at 1.
    pub column: u32,
}

/// An `# unref: ignore[...]` comment, which ignores the findings of its line that have one of its
/// codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IgnoreComment {
    /// The line of the comment, starting at 1.
    pub line: u32,
    /// The column of the `#` of the comment, starting at 1.
    pub column: u32,
    /// The comment from `unref:` to its closing bracket.
    pub text: String,
    /// The codes that the comment names.
    pub codes: Vec<IgnoredCode>,
}

/// An ignore comment that does not parse: the offset in the comment where it stops parsing, and what
/// is wrong.
struct InvalidComment {
    offset: usize,
    message: &'static str,
}

/// Find the `# unref: ignore[...]` comments of a Python module, as ty finds its `# ty: ignore[...]`
/// comments.
///
/// A comment names its codes in brackets, separated by commas, with optional spaces and trailing
/// comma: `# unref: ignore[unused-function, unused-class]`. It can follow another comment on its line,
/// and be followed by one. A comment that does not name any code, or that does not parse, ignores
/// nothing and is a finding, as is a code that is not known.
///
/// Returns the comments that parse, and the findings about the comments.
pub(crate) fn find_ignore_comments(
    text: &str,
    tokens: &Tokens,
    positions: &SourcePositions,
    path: &Path,
) -> (Vec<IgnoreComment>, Vec<Finding>) {
    let mut comments = Vec::new();
    let mut findings = Vec::new();
    // Most modules have no such comment.
    if !text.contains(MARKER) {
        return (comments, findings);
    }

    for token in tokens
        .iter()
        .filter(|token| token.kind() == TokenKind::Comment)
    {
        let comment = &text[token.range()];
        let (line, column) = positions.line_column(token.start());
        let start = column - 1;
        // Offsets in the comment count characters, as columns do.
        let characters: Vec<char> = comment.chars().collect();
        let character_offset = |byte: usize| comment[..byte].chars().count();
        for found in START.find_iter(comment) {
            let (match_start, match_end) = (
                character_offset(found.start()),
                character_offset(found.end()),
            );
            match parse_codes(&characters, match_start, match_end) {
                Err(error) => findings.push(Finding {
                    path: path.to_path_buf(),
                    line,
                    column: start + to_u32(error.offset) + 1,
                    code: Rule::InvalidIgnoreComment,
                    name: error.message.to_string(),
                    transitive: false,
                }),
                Ok((codes, end)) => {
                    let ignored: Vec<IgnoredCode> = codes
                        .into_iter()
                        .map(|(code, offset)| IgnoredCode {
                            code,
                            column: start + to_u32(offset) + 1,
                        })
                        .collect();
                    findings.extend(
                        ignored
                            .iter()
                            .filter(|code| !Rule::is_known(&code.code))
                            .map(|code| Finding {
                                path: path.to_path_buf(),
                                line,
                                column: code.column,
                                code: Rule::UnknownCode,
                                name: code.code.clone(),
                                transitive: false,
                            }),
                    );
                    let text: String = characters[match_start + 1..end].iter().collect();
                    comments.push(IgnoreComment {
                        line,
                        column: start + to_u32(match_start) + 1,
                        text: text.trim_start().to_string(),
                        codes: ignored,
                    });
                }
            }
        }
    }
    (comments, findings)
}

/// Parse the codes of an ignore comment, which follow its `unref: ignore`.
///
/// `start` is the offset of the `#` of the ignore comment in the comment token, which can hold other
/// comments as well, and `offset` the offset just after `ignore`.
///
/// Returns the codes, each with its offset in the token, and the offset just after the closing
/// bracket.
fn parse_codes(
    comment: &[char],
    start: usize,
    offset: usize,
) -> Result<(Vec<(String, usize)>, usize), InvalidComment> {
    let at = |position: usize, character: char| comment.get(position) == Some(&character);
    let mut position = skip_spaces(comment, offset);
    if position == offset && !at(position, '[') && !at(position, '#') && position < comment.len() {
        return Err(InvalidComment {
            offset,
            message: "no whitespace after `ignore`",
        });
    }
    if !at(position, '[') {
        return Err(InvalidComment {
            offset: start,
            message: NO_CODES,
        });
    }

    let mut codes = Vec::new();
    position += 1;
    loop {
        position = skip_spaces(comment, position);
        if at(position, ']') {
            break;
        }
        let end = comment[position..]
            .iter()
            .position(|&character| {
                !(character.is_ascii_alphanumeric() || character == '_' || character == '-')
            })
            .map_or(comment.len(), |length| position + length);
        if end == position {
            return Err(InvalidComment {
                offset: position,
                message: "expected a code",
            });
        }
        codes.push((comment[position..end].iter().collect(), position));
        position = skip_spaces(comment, end);
        if at(position, ']') {
            break;
        }
        if !at(position, ',') {
            return Err(InvalidComment {
                offset: position,
                message: "expected a comma separating the codes",
            });
        }
        position += 1;
    }

    if codes.is_empty() {
        return Err(InvalidComment {
            offset: start,
            message: NO_CODES,
        });
    }
    Ok((codes, position + 1))
}

/// Get the offset of the first character at or after a position that is not a space.
fn skip_spaces(text: &[char], mut position: usize) -> usize {
    while position < text.len() && text[position].is_whitespace() {
        position += 1;
    }
    position
}

/// Get the codes that ignore comments ignore, each with the line that it is ignored on.
pub(crate) fn ignored_codes(comments: &[IgnoreComment]) -> FxHashSet<(u32, &str)> {
    comments
        .iter()
        .flat_map(|comment| {
            comment
                .codes
                .iter()
                .map(|code| (comment.line, code.code.as_str()))
        })
        .collect()
}

/// Apply the ignore comments of a file to its findings about ignore comments, and find the codes of
/// the comments that ignore nothing, as ty does.
///
/// A comment that ignores nothing is a finding at its `#`. A code of a comment that ignores other
/// findings is a finding at the code. A comment can ignore the findings about the comments of its line,
/// including `unused-ignore-comment` for the other codes of its line, but not for itself.
///
/// `ignored` holds the findings of unused definitions that the comments ignore, each as its line and
/// code.
///
/// Returns the findings about the ignore comments that are left, and one for each code that ignores
/// nothing.
pub(crate) fn apply_ignore_comments(
    path: &Path,
    comments: &[IgnoreComment],
    findings: &[Finding],
    ignored: &[(u32, &str)],
) -> Vec<Finding> {
    // Each code of a comment, by the comment and its index, that ignores a finding.
    let mut used: FxHashSet<(usize, usize)> = FxHashSet::default();

    // Mark the last code of the line that ignores a finding as used, and tell whether there is one.
    let ignore = |line: u32, code: &str, used: &mut FxHashSet<(usize, usize)>| -> bool {
        let last = comments
            .iter()
            .enumerate()
            .filter(|(_, comment)| comment.line == line)
            .flat_map(|(comment, item)| {
                item.codes
                    .iter()
                    .enumerate()
                    .filter(|(_, ignored)| ignored.code == code)
                    .map(move |(index, _)| (comment, index))
            })
            .last();
        if let Some(last) = last {
            used.insert(last);
        }
        last.is_some()
    };

    for &(line, code) in ignored {
        ignore(line, code, &mut used);
    }
    let mut left: Vec<Finding> = findings
        .iter()
        .filter(|finding| !ignore(finding.line, finding.code.name(), &mut used))
        .cloned()
        .collect();

    // The known codes that ignore nothing. Those other than `unused-ignore-comment` can be ignored by
    // an `unused-ignore-comment` code of their line, which is then used, so they are found first.
    let codes = || {
        comments.iter().enumerate().flat_map(|(comment, item)| {
            item.codes
                .iter()
                .enumerate()
                .map(move |(index, code)| (comment, index, code.code.as_str()))
        })
    };
    let candidates: Vec<(usize, usize)> = codes()
        .filter(|&(comment, index, code)| {
            !used.contains(&(comment, index))
                && Rule::is_known(code)
                && code != Rule::UnusedIgnoreComment.name()
        })
        .map(|(comment, index, _)| (comment, index))
        .collect();
    let mut unused: Vec<(usize, usize)> = candidates
        .into_iter()
        .filter(|&(comment, _)| {
            !ignore(
                comments[comment].line,
                Rule::UnusedIgnoreComment.name(),
                &mut used,
            )
        })
        .collect();
    unused.extend(
        codes()
            .filter(|&(comment, index, code)| {
                !used.contains(&(comment, index)) && code == Rule::UnusedIgnoreComment.name()
            })
            .map(|(comment, index, _)| (comment, index)),
    );

    // A comment whose codes all ignore nothing is a finding as a whole.
    for (position, comment) in comments.iter().enumerate() {
        let mut indexes: Vec<usize> = unused
            .iter()
            .filter(|(unused_comment, _)| *unused_comment == position)
            .map(|&(_, index)| index)
            .collect();
        indexes.sort_unstable();
        if !indexes.is_empty() && indexes.len() == comment.codes.len() {
            left.push(Finding {
                path: path.to_path_buf(),
                line: comment.line,
                column: comment.column,
                code: Rule::UnusedIgnoreComment,
                name: comment.text.clone(),
                transitive: false,
            });
        } else {
            left.extend(indexes.into_iter().map(|index| Finding {
                path: path.to_path_buf(),
                line: comment.line,
                column: comment.codes[index].column,
                code: Rule::UnusedIgnoreComment,
                name: comment.codes[index].code.clone(),
                transitive: false,
            }));
        }
    }
    left
}
