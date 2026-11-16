//! The Python files of a project.

use std::path::{Component, Path, PathBuf};

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::modules::resolve;

/// The patterns that are excluded by default, before the `exclude` ones. The same as ty's defaults.
const DEFAULT_EXCLUDE: [&str; 20] = [
    "**/.bzr/",
    "**/.direnv/",
    "**/.eggs/",
    "**/.git/",
    "**/.git-rewrite/",
    "**/.hg/",
    "**/.mypy_cache/",
    "**/.nox/",
    "**/.pants.d/",
    "**/.pytype/",
    "**/.ruff_cache/",
    "**/.svn/",
    "**/.tox/",
    "**/.venv/",
    "**/__pypackages__/",
    "**/_build/",
    "**/buck-out/",
    "**/dist/",
    "**/node_modules/",
    "**/venv/",
];

/// The patterns of a `.gitignore` file, with the absolute directory that they are relative to.
type IgnoreFile = (PathBuf, Gitignore);

/// Find the Python files of a project.
///
/// A file in `paths` is taken as is, a directory is searched for Python files. The `exclude` patterns
/// are those of the files and directories that are not part of the project, on top of the default
/// ones, in the syntax of `.gitignore`. They are anchored at the current directory.
///
/// Returns the Python files, sorted.
pub(crate) fn discover(paths: &[PathBuf], exclude: &[String]) -> Vec<PathBuf> {
    let excluded = Exclude::new(exclude);
    let mut files = Vec::new();
    for path in paths {
        let path = clean(path);
        if path.is_dir() {
            walk(&path, &excluded, &mut files);
        } else {
            // A file that is given directly is always checked, as with ruff and ty.
            files.push(path);
        }
    }
    files.sort();
    files.dedup();
    files
}

/// Remove the `.` parts of a path, as Python's `pathlib` does: `./main.py` is `main.py`.
pub(crate) fn clean(path: &Path) -> PathBuf {
    let cleaned: PathBuf = path
        .components()
        .filter(|component| *component != Component::CurDir)
        .collect();
    if cleaned.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        cleaned
    }
}

/// The `exclude` patterns, with the default ones before them, anchored at the current directory.
struct Exclude {
    patterns: Gitignore,
    root: PathBuf,
}

impl Exclude {
    fn new(patterns: &[String]) -> Self {
        let mut builder = GitignoreBuilder::new(".");
        for pattern in DEFAULT_EXCLUDE
            .iter()
            .map(|pattern| (*pattern).to_string())
            .chain(patterns.iter().map(|pattern| anchor(pattern)))
        {
            // An invalid pattern is skipped, as git skips it.
            if is_valid_pattern(&pattern) {
                let _ = builder.add_line(None, &pattern);
            }
        }
        Self {
            patterns: builder.build().unwrap_or_else(|_| Gitignore::empty()),
            root: resolve(Path::new(".")),
        }
    }

    /// Check whether a file or directory under a given directory, which is resolved, is excluded.
    /// The last pattern that matches it decides.
    fn excludes(&self, path: &Path, is_directory: bool, given: &Path) -> bool {
        // A path outside of the current directory can still match the patterns that start with `**/`.
        let relative_to_base = |path: &Path| {
            path.strip_prefix(&self.root)
                .map_or_else(|_| without_root(path), Path::to_path_buf)
        };
        let relative = relative_to_base(&resolve(path));
        let given = relative_to_base(given);
        matches!(
            matched(&self.patterns, &relative, is_directory, &given),
            Match::Ignore(())
        )
    }
}

/// Anchor a pattern at the current directory, as ty anchors them at the project root: `src` matches
/// only the `src` there, `**/src` matches one at any depth.
fn anchor(pattern: &str) -> String {
    let (negation, body) = match pattern.strip_prefix('!') {
        Some(body) => ("!", body),
        None => ("", pattern),
    };
    let body = body.strip_prefix("./").unwrap_or(body);
    if body.starts_with('/') || body.starts_with("**/") {
        format!("{negation}{body}")
    } else {
        format!("{negation}/{body}")
    }
}

/// A path without its root, such as `home/user` for `/home/user`.
fn without_root(path: &Path) -> PathBuf {
    path.components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .collect()
}

/// Add the Python files under a directory. Skips the excluded files and directories, and the ones
/// that git ignores. The directory itself is always walked, even if it is excluded or ignored, as with
/// ruff and ty: only a pattern that matches what is under it counts.
fn walk(directory: &Path, excluded: &Exclude, files: &mut Vec<PathBuf>) {
    let inherited = ignore_files_above(directory);
    let given = resolve(directory);
    walk_directory(directory, excluded, inherited, &given, files);
}

fn walk_directory(
    directory: &Path,
    excluded: &Exclude,
    mut ignore_files: Vec<IgnoreFile>,
    given: &Path,
    files: &mut Vec<PathBuf>,
) {
    ignore_files.extend(read_ignore_file(directory));
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut directories = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        // The files of the current directory are named without a leading `./`, as Python names them.
        let path = if directory == Path::new(".") {
            PathBuf::from(entry.file_name())
        } else {
            directory.join(entry.file_name())
        };
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        // A link to a directory is not walked into, as with Python's `os.walk`.
        let is_directory = kind.is_dir() || (kind.is_symlink() && path.is_dir());
        if is_directory {
            if kind.is_dir()
                && !excluded.excludes(&path, true, given)
                && !is_ignored(&path, &ignore_files, true, given)
            {
                directories.push(path);
            }
        } else if entry.file_name().to_string_lossy().ends_with(".py")
            && !excluded.excludes(&path, false, given)
            && !is_ignored(&path, &ignore_files, false, given)
        {
            files.push(path);
        }
    }
    for child in directories {
        walk_directory(&child, excluded, ignore_files.clone(), given, files);
    }
}

/// Read the `.gitignore` files that apply to a directory from above it: those of its parents, up to
/// the root of its git repository. Outside a git repository no `.gitignore` file applies, as with git.
fn ignore_files_above(directory: &Path) -> Vec<IgnoreFile> {
    let absolute = resolve(directory);
    let Some(root) = absolute
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
    else {
        return Vec::new();
    };
    // The parents from the root of the repository down, without the directory itself.
    let mut parents: Vec<&Path> = absolute
        .ancestors()
        .skip(1)
        .take_while(|parent| parent.starts_with(root))
        .collect();
    parents.reverse();
    parents.into_iter().filter_map(read_ignore_file).collect()
}

/// Read the `.gitignore` file of a directory, if it has one. An invalid line, such as a lone `!`, is
/// skipped and the other lines of the file still apply, as with git and ruff.
fn read_ignore_file(directory: &Path) -> Option<IgnoreFile> {
    let path = directory.join(".gitignore");
    if !path.is_file() {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    let mut builder = GitignoreBuilder::new(".");
    for line in String::from_utf8_lossy(&bytes)
        .lines()
        .filter(|line| is_valid_pattern(line))
    {
        let _ = builder.add_line(None, line);
    }
    Some((resolve(directory), builder.build().ok()?))
}

/// Check whether a line of a `.gitignore` file, or an `exclude` pattern, is valid.
fn is_valid_pattern(line: &str) -> bool {
    // A lone `!` negates nothing, where the `ignore` crate would take it as a pattern that brings
    // everything back.
    if line.trim_end() == "!" {
        return false;
    }
    let mut builder = GitignoreBuilder::new(".");
    builder.add_line(None, line).is_ok() && builder.build().is_ok()
}

/// Check whether git ignores a file or directory. A deeper `.gitignore` file overrides the ones above
/// it, and within a file the last matching pattern wins, so that `!pattern` can bring a path back.
fn is_ignored(path: &Path, ignore_files: &[IgnoreFile], is_directory: bool, given: &Path) -> bool {
    let absolute = resolve(path);
    let mut ignored = false;
    for (base, patterns) in ignore_files {
        let Ok(relative) = absolute.strip_prefix(base) else {
            continue;
        };
        let given = given.strip_prefix(base).unwrap_or(Path::new(""));
        match matched(patterns, relative, is_directory, given) {
            Match::Ignore(()) => ignored = true,
            Match::Whitelist(()) => ignored = false,
            Match::None => {}
        }
    }
    ignored
}

/// Match a path under a given directory against patterns, both relative to the directory that the
/// patterns are relative to.
///
/// A pattern that matches the path only through the given directory or one of its parents does not
/// count, so that a directory given directly is checked even if it is excluded or ignored, as with
/// ruff and ty: `generated/**` excludes nothing when `generated` is given. Such a pattern is found
/// by matching it against a path of the same depth under the given directory whose names no pattern
/// names.
fn matched(patterns: &Gitignore, path: &Path, is_directory: bool, given: &Path) -> Match<()> {
    let found = match patterns.matched(path, is_directory) {
        Match::None => return Match::None,
        Match::Ignore(_) => Match::Ignore(()),
        Match::Whitelist(_) => Match::Whitelist(()),
    };
    let Ok(rest) = path.strip_prefix(given) else {
        return found;
    };
    if given.as_os_str().is_empty() {
        return found;
    }
    let placeholder: PathBuf = given
        .iter()
        .map(Into::into)
        .chain(rest.iter().map(|_| PLACEHOLDER.into()))
        .collect::<Vec<std::ffi::OsString>>()
        .iter()
        .collect();
    if patterns.matched(&placeholder, is_directory).is_none() {
        found
    } else {
        Match::None
    }
}

/// A name that no pattern names, see `matched`.
const PLACEHOLDER: &str = "\u{1}unref";

/// Check whether a path is one of the given files, or is under one of the given directories, which
/// are resolved.
pub(crate) fn is_under(path: &Path, roots: &[PathBuf]) -> bool {
    let absolute = resolve(path);
    roots.iter().any(|root| absolute.starts_with(root))
}

#[cfg(test)]
mod tests {
    use super::{anchor, clean};
    use std::path::{Path, PathBuf};

    #[test]
    fn anchors() {
        assert_eq!(anchor("src"), "/src");
        assert_eq!(anchor("./src"), "/src");
        assert_eq!(anchor("**/src"), "**/src");
        assert_eq!(anchor("!src"), "!/src");
        assert_eq!(anchor("/src"), "/src");
    }

    #[test]
    fn cleans() {
        assert_eq!(clean(Path::new("./main.py")), PathBuf::from("main.py"));
        assert_eq!(clean(Path::new(".")), PathBuf::from("."));
        assert_eq!(clean(Path::new("a/./b/")), PathBuf::from("a/b"));
    }
}
