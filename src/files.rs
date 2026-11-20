//! What each Python file defines and refers to, found for every file in parallel.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use rayon::prelude::*;
use regex::bytes::Regex;
use ruff_python_ast::{CmpOp, Expr, ModModule, Stmt};
use ruff_python_parser::Parsed;
use ruff_text_size::{Ranged, TextSize};
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

/// Find what a Python file defines and refers to, or a finding if it cannot be read or does not parse.
pub(crate) fn analyze_file(path: &Arc<Path>) -> Result<FileFacts, Finding> {
    let syntax_error = |line: u32, column: u32, name: String| {
        Finding::new(path.to_path_buf(), line, column, Rule::InvalidSyntax, name)
    };
    // A file that cannot be read, such as a given path that does not exist, has its own code, as
    // ruff's `io-error`. A file that is read but cannot be decoded is a syntax error, as in Python.
    let bytes = std::fs::read(path).map_err(|error| Finding {
        code: Rule::IoError,
        ..syntax_error(1, 1, error.to_string())
    })?;
    let text = decode_source(&bytes)
        .map_err(|error| syntax_error(error.line, error.column, error.message))?;
    let positions = SourcePositions::new(&text);
    let parsed = ruff_python_parser::parse_module(&text).map_err(|error| {
        let (line, column) = positions.line_column(error.location.start());
        syntax_error(line, column, error.error.to_string())
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
    pub message: String,
}

/// Decode the source of a module as Python does: UTF-8, or the encoding that it declares (PEP 263).
/// A byte order mark is dropped, and then only UTF-8 can be declared.
pub(crate) fn decode_source(bytes: &[u8]) -> Result<String, DecodeError> {
    let (bom, bytes) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (true, rest),
        None => (false, bytes),
    };
    let error = |message: String| DecodeError {
        line: 1,
        column: 1,
        message,
    };
    let Some(declared) = declared_encoding(bytes) else {
        return utf8(bytes);
    };
    let name = declared.to_lowercase().replace('_', "-");
    let is_utf8 = name == "utf-8" || name.starts_with("utf-8-");
    if bom && !is_utf8 {
        return Err(error(format!("encoding problem: {declared} with BOM")));
    }
    // Python takes `latin-1-unix` and the like as Latin-1.
    let is_latin1 = ["latin-1", "iso-8859-1", "iso-latin-1"]
        .iter()
        .any(|latin1| name == *latin1 || name.starts_with(&format!("{latin1}-")))
        || ["latin1", "iso8859-1", "l1"].contains(&name.as_str());
    match name.as_str() {
        _ if is_utf8 => utf8(bytes),
        "utf8" => utf8(bytes),
        // Latin-1 maps each byte to the character with its value, where the WHATWG encodings that
        // `encoding_rs` implements map it to Windows-1252.
        _ if is_latin1 => Ok(bytes.iter().map(|&byte| char::from(byte)).collect()),
        // ASCII is strict, where WHATWG takes it as Windows-1252.
        "ascii" | "us-ascii" | "646" => match bytes.iter().position(|byte| !byte.is_ascii()) {
            None => Ok(bytes.iter().map(|&byte| char::from(byte)).collect()),
            Some(_) => Err(error(format!(
                "(unicode error) '{declared}' codec can't decode the source"
            ))),
        },
        _ => {
            let encoding = encoding_rs::Encoding::for_label(name.as_bytes())
                .ok_or_else(|| error(format!("unknown encoding: {declared}")))?;
            let (text, _, had_errors) = encoding.decode(bytes);
            if had_errors {
                return Err(error(format!(
                    "(unicode error) '{declared}' codec can't decode the source"
                )));
            }
            Ok(text.into_owned())
        }
    }
}

/// The encoding that a module declares on one of its first two lines, if it declares one.
fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let mut lines = bytes.split(|&byte| byte == b'\n');
    let first = lines.next().unwrap_or_default();
    let mut declared = CODING.captures(first);
    // The declaration can be on the second line only if the first line is a comment or blank.
    if declared.is_none() {
        let trimmed = first.trim_ascii_start();
        if trimmed.is_empty() || trimmed.starts_with(b"#") {
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
        let (line, column) =
            SourcePositions::new(&before).line_column(TextSize::new(to_u32(position)));
        DecodeError {
            line,
            column,
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
