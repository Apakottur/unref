//! The findings of a check, and the options that apply to some of the definitions of a project.

use std::fmt;
use std::path::PathBuf;

/// The default of the `dummy-variable-rgx` option, the same as Ruff's.
pub(crate) const DEFAULT_DUMMY_VARIABLE_RGX: &str = r"^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$";

/// Define `Framework`, with a variant for each framework and its name in the `frameworks` option, and
/// `Framework::ALL`, in the order given.
macro_rules! frameworks {
    ($($framework:ident => $name:literal,)*) => {
        /// A framework whose built-in rules can apply, by its name in the `frameworks` option.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub(crate) enum Framework {
            $($framework,)*
        }

        impl Framework {
            /// Every framework, each enabled by default.
            pub(crate) const ALL: &[Framework] = &[$(Framework::$framework,)*];

            /// The name of the framework in the `frameworks` option.
            pub(crate) fn name(self) -> &'static str {
                match self {
                    $(Framework::$framework => $name,)*
                }
            }
        }
    };
}

frameworks! {
    Pytest => "pytest",
    Unittest => "unittest",
    Stdlib => "stdlib",
    Pydantic => "pydantic",
    Dataclasses => "dataclasses",
    Sqlalchemy => "sqlalchemy",
    Fastapi => "fastapi",
    Flask => "flask",
    FlaskRestful => "flask-restful",
    Sphinx => "sphinx",
    Django => "django",
    Drf => "rest-framework",
    Celery => "celery",
    Click => "click",
    Typer => "typer",
}

impl Framework {
    /// The framework of a name in the `frameworks` option.
    pub(crate) fn from_name(name: &str) -> Option<Framework> {
        Framework::ALL
            .iter()
            .copied()
            .find(|framework| framework.name() == name)
    }
}

/// The rule of a finding, by its code, which an ignore comment can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Rule {
    UnusedFunction,
    UnusedClass,
    UnusedMethod,
    UnusedVariable,
    UnusedAttribute,
    UnusedIgnoreComment,
    InvalidIgnoreComment,
    UnknownCode,
    InvalidSyntax,
    IoError,
}

impl Rule {
    /// Every rule.
    pub(crate) const ALL: [Rule; 10] = [
        Rule::UnusedFunction,
        Rule::UnusedClass,
        Rule::UnusedMethod,
        Rule::UnusedVariable,
        Rule::UnusedAttribute,
        Rule::UnusedIgnoreComment,
        Rule::InvalidIgnoreComment,
        Rule::UnknownCode,
        Rule::InvalidSyntax,
        Rule::IoError,
    ];

    /// The code of the rule, such as `unused-function`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Rule::UnusedFunction => "unused-function",
            Rule::UnusedClass => "unused-class",
            Rule::UnusedMethod => "unused-method",
            Rule::UnusedVariable => "unused-variable",
            Rule::UnusedAttribute => "unused-attribute",
            Rule::UnusedIgnoreComment => "unused-ignore-comment",
            Rule::InvalidIgnoreComment => "invalid-ignore-comment",
            Rule::UnknownCode => "ignore-comment-unknown-rule",
            Rule::InvalidSyntax => "invalid-syntax",
            Rule::IoError => "io-error",
        }
    }

    /// Whether a code that an ignore comment names is the code of a rule.
    pub(crate) fn is_known(code: &str) -> bool {
        Rule::ALL.into_iter().any(|rule| rule.name() == code)
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// An unused definition, a problem with an ignore comment, or a file that cannot be read or does not
/// parse.
///
/// Its `Display` gives the line that the command prints, `path:line:column: code 'name'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// The file of the finding.
    pub path: PathBuf,
    /// The line of the finding, starting at 1.
    pub line: u32,
    /// The column of the finding, starting at 1, as ruff and ty count it.
    pub column: u32,
    /// The rule of the finding, shown by its code, such as `unused-function`.
    pub code: Rule,
    /// The name of the unused definition, or what is wrong.
    pub name: String,
    /// Whether the unused definition is reached only from other unused definitions, which are
    /// reported too. The command shows it only with `show-transitive`.
    pub transitive: bool,
}

impl Finding {
    /// Make a finding that is not transitive.
    pub(crate) fn new(path: PathBuf, line: u32, column: u32, code: Rule, name: String) -> Self {
        Self {
            path,
            line,
            column,
            code,
            name,
            transitive: false,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {} '{}'",
            self.path.display(),
            self.line,
            self.column,
            self.code,
            self.name
        )
    }
}

/// Options that apply only to some of the definitions of a project, on top of those of the project.
///
/// An override needs at least one selector, and applies to what matches all of its selectors.
#[derive(Debug, Clone)]
pub(crate) struct Override {
    /// Selects the definitions in these files and directories.
    pub paths: Vec<PathBuf>,
    /// Selects the definitions in subclasses of these classes, such as `ast.NodeVisitor`.
    pub base_classes: Vec<String>,
    /// Selects the definitions in classes with these decorators, such as `plugins.register`.
    pub class_decorators: Vec<String>,
    /// Selects the definitions in classes whose names match these patterns, such as `*Values`.
    pub class_names: Vec<String>,
    /// Names, as patterns, of the selected definitions that count as used.
    pub used_names: Vec<String>,
    /// Decorators that make the selected definitions they decorate count as used.
    pub used_decorators: Vec<String>,
}

/// Check whether a name matches a pattern of the shell, as Python's `fnmatch.fnmatchcase` does: `*`
/// matches anything, `?` any one character, `[seq]` any character in `seq` and `[!seq]` any other.
pub(crate) fn fnmatch(name: &str, pattern: &str) -> bool {
    let name: Vec<char> = name.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    fnmatch_chars(&name, &pattern)
}

fn fnmatch_chars(name: &[char], pattern: &[char]) -> bool {
    // The position in the pattern after the last `*`, and the position in the name that it matched up to.
    let mut star: Option<(usize, usize)> = None;
    let (mut n, mut p) = (0, 0);
    while n < name.len() {
        if p < pattern.len() {
            match pattern[p] {
                '*' => {
                    star = Some((p + 1, n));
                    p += 1;
                    continue;
                }
                '?' => {
                    n += 1;
                    p += 1;
                    continue;
                }
                '[' => {
                    if let Some((matched, end)) = match_class(name[n], &pattern[p..]) {
                        if matched {
                            n += 1;
                            p += end;
                            continue;
                        }
                    } else if name[n] == '[' {
                        // A `[` that does not open a class matches itself.
                        n += 1;
                        p += 1;
                        continue;
                    }
                }
                character if character == name[n] => {
                    n += 1;
                    p += 1;
                    continue;
                }
                _ => {}
            }
        }
        match star {
            Some((after, matched)) => {
                p = after;
                n = matched + 1;
                star = Some((after, matched + 1));
            }
            None => return false,
        }
    }
    pattern[p..].iter().all(|&character| character == '*')
}

/// Match a character against a class that starts a pattern, such as `[a-z]`.
///
/// Returns whether it matches and the length of the class, or None if the `[` does not open a class.
fn match_class(character: char, pattern: &[char]) -> Option<(bool, usize)> {
    let mut index = 1;
    let negated = matches!(pattern.get(index), Some('!'));
    if negated {
        index += 1;
    }
    // A `]` right after the opening is a character of the class.
    let start = index;
    while index < pattern.len() && (pattern[index] != ']' || index == start) {
        index += 1;
    }
    if index >= pattern.len() {
        return None;
    }
    let class = &pattern[start..index];
    let mut matched = false;
    let mut item = 0;
    while item < class.len() {
        if item + 2 < class.len() && class[item + 1] == '-' {
            if class[item] <= character && character <= class[item + 2] {
                matched = true;
            }
            item += 3;
        } else {
            if class[item] == character {
                matched = true;
            }
            item += 1;
        }
    }
    Some((matched != negated, index + 1))
}

#[cfg(test)]
mod tests {
    use super::fnmatch;

    #[test]
    fn patterns() {
        assert!(fnmatch("test_one.py", "test_*.py"));
        assert!(fnmatch("one_test.py", "*_test.py"));
        assert!(!fnmatch("one.py", "test*.py"));
        assert!(fnmatch("handle", "hand?e"));
        assert!(fnmatch("visit_Name", "visit_[A-Z]*"));
        assert!(!fnmatch("visit_name", "visit_[!a-z]*"));
        assert!(fnmatch("a[b", "a[b"));
        assert!(fnmatch("", "*"));
        assert!(!fnmatch("abc", "a*d"));
    }
}
