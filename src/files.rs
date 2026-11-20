//! What each Python file defines and refers to, found for every file in parallel.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use rayon::prelude::*;
use regex::bytes::Regex;
use ruff_python_ast::{CmpOp, Expr, ModModule, Stmt};
use ruff_python_parser::Parsed;
use ruff_text_size::Ranged;
use rustc_hash::FxHashMap;

use crate::definitions::Definition;
use crate::ignores::{IgnoreComment, find_ignore_comments};
use crate::models::{Finding, Rule};
use crate::positions::{SourcePositions, to_u32};
use crate::references::{Binding, References, walk_module};

/// The encoding declaration of a module (PEP 263), on one of its first two lines.
static CODING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[ \t\x0c]*#.*?coding[:=][ \t]*([-\w.]+)").expect("valid pattern")
});

/// The characters for bytes 0x80 through 0xFF in Python's CP437 codec. Lower bytes are ASCII.
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

/// What a Python file defines and refers to.
#[derive(Debug)]
pub(crate) struct FileFacts {
    /// The definitions of the file.
    pub definitions: Vec<Definition>,
    /// What the file refers to.
    pub references: References,
    /// The ignore comments of the file that parse.
    pub ignore_comments: Vec<IgnoreComment>,
    /// The findings about the ignore comments of the file, which they can ignore themselves.
    pub ignore_findings: Vec<Finding>,
    /// The names that the file gives out from its `__getattr__`, each with the imports of it.
    pub lazy: FxHashMap<String, Vec<Binding>>,
    /// Whether the file is run as a script: it checks `if __name__ == "__main__":` at its top level.
    pub is_script: bool,
}

/// Find what each Python file defines and refers to, or a finding for a file that does not parse.
///
/// Each file is analyzed on its own, in the thread pool that the call runs in.
pub(crate) fn analyze_files(paths: &[Arc<Path>]) -> Vec<Result<FileFacts, Finding>> {
    paths.par_iter().map(analyze_file).collect()
}

/// Find what a Python file defines and refers to, or report a read, decoding or parse error.
pub(crate) fn analyze_file(path: &Arc<Path>) -> Result<FileFacts, Finding> {
    let finding =
        |line, column, rule, message| Finding::new(path.to_path_buf(), line, column, rule, message);
    let bytes =
        std::fs::read(path).map_err(|error| finding(1, 1, Rule::IoError, error.to_string()))?;
    let text = decode_source(&bytes)
        .map_err(|error| finding(error.line, error.column, error.rule, error.message))?;
    let positions = SourcePositions::new(&text);
    let parsed = ruff_python_parser::parse_module(&text).map_err(|error| {
        let (line, column) = positions.line_column(error.location.start());
        finding(line, column, Rule::InvalidSyntax, error.error.to_string())
    })?;

    let (definitions, references) = walk_module(path, parsed.syntax(), &positions);
    let (ignore_comments, ignore_findings) =
        find_ignore_comments(&text, parsed.tokens(), &positions, path);
    let lazy = lazy_imports(&parsed, &positions, &references);
    Ok(FileFacts {
        definitions,
        references,
        ignore_comments,
        ignore_findings,
        lazy,
        is_script: is_script(parsed.syntax()),
    })
}

/// Check whether a module checks `if __name__ == "__main__":` at its top level, in either order.
fn is_script(module: &ModModule) -> bool {
    module.body.iter().any(|stmt| {
        let Stmt::If(stmt_if) = stmt else {
            return false;
        };
        let Expr::Compare(compare) = &*stmt_if.test else {
            return false;
        };
        let [CmpOp::Eq] = &*compare.ops else {
            return false;
        };
        let [left, right] = &*compare.operands else {
            return false;
        };
        let is_name = |expr: &Expr| matches!(expr, Expr::Name(name) if name.id == "__name__");
        let is_main = |expr: &Expr| {
            matches!(expr, Expr::StringLiteral(literal) if literal.value.to_str() == "__main__")
        };
        (is_name(left) && is_main(right)) || (is_main(left) && is_name(right))
    })
}

/// Why the source of a module cannot be decoded, with the line and the column where it fails.
#[derive(Debug)]
pub(crate) struct DecodeError {
    pub line: u32,
    pub column: u32,
    rule: Rule,
    pub message: String,
}

/// Decode the source of a module as Python does: UTF-8, or the encoding that it declares (PEP 263).
/// A byte order mark is dropped, and then only UTF-8 can be declared.
pub(crate) fn decode_source(bytes: &[u8]) -> Result<String, DecodeError> {
    let (bom, bytes) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (true, rest),
        None => (false, bytes),
    };
    let error = |rule, message| DecodeError {
        line: 1,
        column: 1,
        rule,
        message,
    };
    let Some(declared) = declared_encoding(bytes) else {
        return utf8(bytes);
    };
    let name = declared.to_lowercase().replace('_', "-");
    let is_utf8 = name == "utf-8" || name.starts_with("utf-8-");
    if bom && !is_utf8 {
        return Err(error(
            Rule::InvalidSyntax,
            format!("encoding problem: {declared} with BOM"),
        ));
    }
    // Python takes `latin-1-unix` and the like as Latin-1.
    let is_latin1 = ["latin-1", "iso-8859-1", "iso-latin-1"]
        .iter()
        .any(|latin1| name == *latin1 || name.starts_with(&format!("{latin1}-")))
        || ["latin1", "iso8859-1", "l1"].contains(&name.as_str());
    // Python ignores surrounding dashes and underscores when looking up these codec names.
    let is_cp437 = ["cp437", "437", "ibm437", "cspc8codepage437"].contains(&name.trim_matches('-'));
    match name.as_str() {
        _ if is_utf8 => utf8(bytes),
        "utf8" => utf8(bytes),
        // Latin-1 maps each byte to the character with its value, where the WHATWG encodings that
        // `encoding_rs` implements map it to Windows-1252.
        _ if is_latin1 => Ok(bytes.iter().map(|&byte| char::from(byte)).collect()),
        // ASCII is strict, where WHATWG takes it as Windows-1252.
        "ascii" | "us-ascii" | "646" => match bytes.iter().position(|byte| !byte.is_ascii()) {
            None => Ok(bytes.iter().map(|&byte| char::from(byte)).collect()),
            Some(_) => Err(error(
                Rule::InvalidSyntax,
                format!("(unicode error) '{declared}' codec can't decode the source"),
            )),
        },
        _ if is_cp437 => Ok(bytes
            .iter()
            .map(|&byte| match byte {
                0..=0x7F => char::from(byte),
                _ => CP437_HIGH[usize::from(byte - 0x80)],
            })
            .collect()),
        _ => {
            let encoding = encoding_rs::Encoding::for_label_no_replacement(name.as_bytes())
                .ok_or_else(|| {
                    error(
                        Rule::UnsupportedEncoding,
                        format!("unsupported encoding: {declared}"),
                    )
                })?;
            let (text, _, had_errors) = encoding.decode(bytes);
            if had_errors {
                return Err(error(
                    Rule::InvalidSyntax,
                    format!("(unicode error) '{declared}' codec can't decode the source"),
                ));
            }
            Ok(text.into_owned())
        }
    }
}

/// The encoding that a module declares on one of its first two lines, if it declares one.
fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let mut lines = bytes.split(|&byte| matches!(byte, b'\r' | b'\n'));
    let first = lines.next().unwrap_or_default();
    let mut declared = CODING.captures(first);
    // The declaration can be on the second line only if the first line is a comment or blank.
    if declared.is_none() {
        let trimmed = first.trim_ascii_start();
        if trimmed.is_empty() || trimmed.starts_with(b"#") {
            // CRLF is one physical line ending, but the split yields an empty slice between them.
            if bytes[first.len()..].starts_with(b"\r\n") {
                lines.next();
            }
            declared = lines.next().and_then(|second| CODING.captures(second));
        }
    }
    declared.map(|declared| String::from_utf8_lossy(&declared[1]).into_owned())
}

/// Decode UTF-8, or find the first byte that is not valid.
fn utf8(bytes: &[u8]) -> Result<String, DecodeError> {
    String::from_utf8(bytes.to_vec()).map_err(|error| {
        let position = error.utf8_error().valid_up_to();
        // The bytes before the invalid one are valid, so they are counted as characters.
        let before = String::from_utf8_lossy(&bytes[..position]);
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        DecodeError {
            line: to_u32(before.matches('\n').count() + 1),
            column: to_u32(before[line_start..].chars().count() + 1),
            rule: Rule::InvalidSyntax,
            message: format!(
                "(unicode error) 'utf-8' codec can't decode byte {:#04x}",
                bytes[position]
            ),
        }
    })
}

/// Get the names that a module gives out from its `__getattr__` (PEP 562), which imports them when
/// they are first read, such as `ThreadPoolExecutor` in `concurrent.futures`.
///
/// A name is given out when the `__getattr__` of the module imports it with `from module import name`.
/// The code of the function is not run, so the condition that the import is under is not checked.
///
/// Returns the names, each with the imports of it in `__getattr__`, as the module imports them.
fn lazy_imports(
    parsed: &Parsed<ModModule>,
    positions: &SourcePositions,
    references: &References,
) -> FxHashMap<String, Vec<Binding>> {
    // The last definition of `__getattr__` at the top level of the module is the one that counts.
    let line = parsed
        .syntax()
        .body
        .iter()
        .rev()
        .find_map(|stmt| match stmt {
            Stmt::FunctionDef(function) if function.name.as_str() == "__getattr__" => {
                Some(positions.line_column(function.name.start()).0)
            }
            _ => None,
        });
    let mut lazy: FxHashMap<String, Vec<Binding>> = FxHashMap::default();
    let Some(line) = line else {
        return lazy;
    };
    for import in &references.imports {
        if import.owner == Some(line) && import.name != "*" {
            lazy.entry(import.name.clone()).or_default().push(Binding {
                module: import.module.clone(),
                level: import.level,
                name: Some(import.name.clone()),
            });
        }
    }
    lazy
}

#[cfg(test)]
mod tests {
    use super::decode_source;

    #[test]
    fn encodings() {
        assert_eq!(decode_source(b"x = 1\n").unwrap(), "x = 1\n");
        assert_eq!(decode_source(&[0xEF, 0xBB, 0xBF, b'x']).unwrap(), "x");
        assert_eq!(
            decode_source(b"# -*- coding: latin-1 -*-\n\"\xe9\"\n").unwrap(),
            "# -*- coding: latin-1 -*-\n\"\u{e9}\"\n"
        );
        assert_eq!(
            decode_source(b"#!/usr/bin/env python\n# coding=latin-1\n\"\x80\"\n").unwrap(),
            "#!/usr/bin/env python\n# coding=latin-1\n\"\u{80}\"\n"
        );
        assert!(decode_source(b"# coding: ascii\n\"\xe9\"\n").is_err());
        assert_eq!(
            decode_source(b"# coding: latin-1-unix\n\"\xe9\"\n").unwrap(),
            "# coding: latin-1-unix\n\"\u{e9}\"\n"
        );
        let error = decode_source(b"x = 1\ny = \"\xe9\"\n").err().unwrap();
        assert_eq!((error.line, error.column), (2, 6));
        assert!(
            decode_source(&[
                0xEF, 0xBB, 0xBF, b'#', b' ', b'c', b'o', b'd', b'i', b'n', b'g', b':', b'l', b'1'
            ])
            .is_err()
        );
    }
}
