//! The configuration of a check, from `pyproject.toml` or another TOML file.

use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use toml::{Table, Value};

use crate::models::{DEFAULT_DUMMY_VARIABLE_RGX, Framework, Override};

const PYPROJECT: &str = "pyproject.toml";

/// The options of an override that select what it applies to.
const SELECTORS: [&str; 3] = ["paths", "base-classes", "class-decorators"];

/// How the findings are printed, named as ruff's `output-format` option names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    /// One line for each finding: `path:line:column: code 'name'`.
    #[default]
    Concise,
    /// A GitHub Actions workflow command for each finding, which shows it as an annotation.
    Github,
}

/// The configuration of a check.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    /// The names, as patterns, of the definitions that count as used.
    pub used_names: Vec<String>,
    /// The decorators that make the definitions they decorate count as used.
    pub used_decorators: Vec<String>,
    /// The patterns of the files and directories that are not part of the project.
    pub exclude: Vec<String>,
    /// Whether the files and directories that are given directly are left out too when excluded.
    pub force_exclude: bool,
    /// Whether the files and directories that git ignores are left out.
    pub respect_gitignore: bool,
    /// The options that apply only to some of the files of the project.
    pub overrides: Vec<Override>,
    /// The Python environment or interpreter of the project, or None if there is none.
    pub python: Option<PathBuf>,
    /// The pattern of the names of the variables that are not used on purpose.
    pub dummy_variable_rgx: String,
    /// The object references of the entry points of the project, such as `package.module:function`.
    pub entry_points: Vec<String>,
    /// Whether the rules of each framework apply, for the frameworks that the file sets.
    pub frameworks: FxHashMap<Framework, bool>,
    /// The configuration files of Sphinx, or None to find them by their contents.
    pub sphinx_configuration: Option<Vec<PathBuf>>,
    /// The settings modules of Django that are given, besides those that are found.
    pub django_settings: Vec<PathBuf>,
    /// How the findings are printed.
    pub output_format: OutputFormat,
    /// Whether the unused definitions that only other unused definitions reach are printed too.
    pub show_transitive: bool,
    /// The directory that the paths of the configuration are relative to, and that is checked when no
    /// paths are given: the directory of a `pyproject.toml` that is found, or else the current one.
    pub root: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            used_names: Vec::new(),
            used_decorators: Vec::new(),
            exclude: Vec::new(),
            force_exclude: false,
            respect_gitignore: true,
            overrides: Vec::new(),
            python: None,
            dummy_variable_rgx: DEFAULT_DUMMY_VARIABLE_RGX.to_string(),
            entry_points: Vec::new(),
            frameworks: FxHashMap::default(),
            sphinx_configuration: None,
            django_settings: Vec::new(),
            output_format: OutputFormat::Concise,
            show_transitive: false,
            root: PathBuf::from("."),
        }
    }
}

/// Load the configuration from a TOML file.
///
/// A file named `pyproject.toml` holds the options in its `[tool.unref]` table, and the entry points
/// of the project in its `[project]` table. Any other file holds the options at the top level. The
/// paths of a given file are relative to the current directory, as with ruff.
///
/// Without a file, the closest `pyproject.toml` with a `[tool.unref]` table is read, from the current
/// directory up, or else the closest `pyproject.toml`, as ty finds its project. Its directory is the
/// root of the project, and its paths are relative to it.
///
/// Returns an error message if the file cannot be read or is not valid.
pub(crate) fn load_config(path: Option<&Path>) -> Result<Config, String> {
    let (path, root) = if let Some(path) = path {
        (path.to_path_buf(), None)
    } else {
        match find_pyproject() {
            Some((path, root)) => (path, Some(root)),
            None => return Ok(Config::default()),
        }
    };
    let mut config = read_config(&path)?;
    if let Some(root) = root {
        for item in &mut config.overrides {
            item.paths = item.paths.iter().map(|path| root.join(path)).collect();
        }
        config.python = config.python.map(|python| root.join(python));
        if let Some(files) = &mut config.sphinx_configuration {
            *files = files.iter().map(|file| root.join(file)).collect();
        }
        config.django_settings = config
            .django_settings
            .iter()
            .map(|file| root.join(file))
            .collect();
        config.root = root;
    }
    Ok(config)
}

/// Find the `pyproject.toml` of the project, from the current directory up: the closest one with a
/// `[tool.unref]` table, or else the closest one. A file that cannot be read or parsed is taken, so
/// that its error is reported.
///
/// Returns the file and its directory, relative to the current directory, or None if there is none.
fn find_pyproject() -> Option<(PathBuf, PathBuf)> {
    let current = std::env::current_dir().ok()?;
    let mut closest = None;
    for (depth, directory) in current.ancestors().enumerate() {
        let candidate = directory.join(PYPROJECT);
        if !candidate.is_file() {
            continue;
        }
        let root: PathBuf = if depth == 0 {
            PathBuf::from(".")
        } else {
            std::iter::repeat_n("..", depth).collect()
        };
        let found = (
            if depth == 0 {
                PathBuf::from(PYPROJECT)
            } else {
                root.join(PYPROJECT)
            },
            root,
        );
        let has_table = std::fs::read_to_string(&candidate)
            .ok()
            .and_then(|text| text.parse::<Table>().ok())
            .is_none_or(|options| {
                options
                    .get("tool")
                    .and_then(|tool| tool.get("unref"))
                    .is_some()
            });
        if has_table {
            return Some(found);
        }
        closest.get_or_insert(found);
    }
    closest
}

/// Read the configuration from a TOML file, with its paths as they are written.
fn read_config(path: &Path) -> Result<Config, String> {
    let shown = path.display();

    let text = std::fs::read(path).map_err(|error| {
        format!(
            "Cannot read the configuration file {shown}: {}",
            strerror(&error)
        )
    })?;
    let text = String::from_utf8(text)
        .map_err(|error| format!("Invalid TOML in the configuration file {shown}: {error}"))?;
    let mut options: Table = text.parse().map_err(|error: toml::de::Error| {
        format!(
            "Invalid TOML in the configuration file {shown}: {}",
            error.message()
        )
    })?;

    let mut entry_points = Vec::new();
    if path.file_name().is_some_and(|name| name == PYPROJECT) {
        let project = options
            .remove("project")
            .unwrap_or_else(|| Value::Table(Table::new()));
        entry_points = parse_entry_points(&project, path)?;
        let place = format!("the configuration file {shown}");
        options = match options.remove("tool") {
            None => Table::new(),
            Some(Value::Table(mut tool)) => match tool.remove("unref") {
                None => Table::new(),
                Some(Value::Table(unref)) => unref,
                Some(_) => return Err(format!("Table 'tool.unref' in {place} must be a table")),
            },
            Some(_) => return Err(format!("Table 'tool' in {place} must be a table")),
        };
    }

    let mut config = parse(&options, path)?;
    config.entry_points = entry_points;
    Ok(config)
}

/// The message of an error of the operating system, without its number, as Python's `strerror`.
fn strerror(error: &std::io::Error) -> String {
    let message = error.to_string();
    match message.rfind(" (os error ") {
        Some(index) => message[..index].to_string(),
        None => message,
    }
}

/// Get the object references of the entry points that the `[project]` table of a `pyproject.toml`
/// gives.
fn parse_entry_points(project: &Value, path: &Path) -> Result<Vec<String>, String> {
    let place = format!("the configuration file {}", path.display());
    let Value::Table(project) = project else {
        return Err(format!("Table 'project' in {place} must be a table"));
    };

    let empty = Value::Table(Table::new());
    let mut tables: Vec<(String, &Value)> = ["scripts", "gui-scripts"]
        .iter()
        .map(|name| {
            (
                format!("project.{name}"),
                project.get(*name).unwrap_or(&empty),
            )
        })
        .collect();
    let groups = project.get("entry-points").unwrap_or(&empty);
    let Value::Table(groups) = groups else {
        return Err(format!(
            "Table 'project.entry-points' in {place} must be a table"
        ));
    };
    tables.extend(
        groups
            .iter()
            .map(|(group, table)| (format!("project.entry-points.{group}"), table)),
    );

    let mut references = Vec::new();
    for (name, table) in tables {
        let invalid = || format!("Table '{name}' in {place} must map names to strings");
        let Value::Table(table) = table else {
            return Err(invalid());
        };
        for value in table.values() {
            let Value::String(reference) = value else {
                return Err(invalid());
            };
            references.push(reference.clone());
        }
    }
    Ok(references)
}

/// Build the configuration from the options of a configuration file.
fn parse(options: &Table, path: &Path) -> Result<Config, String> {
    let place = format!("the configuration file {}", path.display());
    check_names(
        options,
        &[
            "used-names",
            "used-decorators",
            "exclude",
            "force-exclude",
            "respect-gitignore",
            "overrides",
            "python",
            "dummy-variable-rgx",
            "frameworks",
            "output-format",
            "show-transitive",
        ],
        &place,
    )?;

    let invalid_overrides = || format!("Option 'overrides' in {place} must be a list of tables");
    let overrides = match options.get("overrides") {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_table().ok_or_else(invalid_overrides))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(invalid_overrides()),
    };

    let python = string_option(options, "python", &place)?.map(PathBuf::from);

    let dummy_variable_rgx = string_option(options, "dummy-variable-rgx", &place)?
        .unwrap_or(DEFAULT_DUMMY_VARIABLE_RGX)
        .to_string();
    if let Err(error) = fancy_regex::Regex::new(&dummy_variable_rgx) {
        return Err(format!(
            "Option 'dummy-variable-rgx' in {place} is not a valid regular expression: {error}"
        ));
    }

    let output_format = match options.get("output-format").map(Value::as_str) {
        None | Some(Some("concise")) => OutputFormat::Concise,
        Some(Some("github")) => OutputFormat::Github,
        Some(_) => {
            return Err(format!(
                "Option 'output-format' in {place} must be one of 'concise', 'github'"
            ));
        }
    };

    let show_transitive = bool_option(options, "show-transitive", &place)?.unwrap_or(false);
    let force_exclude = bool_option(options, "force-exclude", &place)?.unwrap_or(false);
    let respect_gitignore = bool_option(options, "respect-gitignore", &place)?.unwrap_or(true);

    let Frameworks {
        enabled: frameworks,
        sphinx_configuration,
        django_settings,
    } = parse_frameworks(options, &place)?;

    Ok(Config {
        used_names: string_list(options, "used-names", &place)?,
        used_decorators: decorators(options, "used-decorators", &place)?,
        exclude: string_list(options, "exclude", &place)?,
        force_exclude,
        respect_gitignore,
        overrides: overrides
            .into_iter()
            .map(|table| parse_override(table, path))
            .collect::<Result<_, _>>()?,
        python,
        dummy_variable_rgx,
        frameworks,
        sphinx_configuration,
        django_settings,
        output_format,
        show_transitive,
        ..Config::default()
    })
}

/// Build an override from one of the `overrides` tables of a configuration file.
fn parse_override(options: &Table, path: &Path) -> Result<Override, String> {
    let place = format!("an override in the configuration file {}", path.display());
    check_names(
        options,
        &[
            "paths",
            "base-classes",
            "class-decorators",
            "used-names",
            "used-decorators",
        ],
        &place,
    )?;

    if SELECTORS
        .iter()
        .all(|selector| !options.contains_key(*selector))
    {
        let selectors: Vec<String> = SELECTORS
            .iter()
            .map(|selector| format!("'{selector}'"))
            .collect();
        return Err(format!(
            "An override in the configuration file {} needs one of the options {}",
            path.display(),
            selectors.join(", ")
        ));
    }

    Ok(Override {
        paths: string_list(options, "paths", &place)?
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        base_classes: string_list(options, "base-classes", &place)?,
        class_decorators: decorators(options, "class-decorators", &place)?,
        used_names: string_list(options, "used-names", &place)?,
        used_decorators: decorators(options, "used-decorators", &place)?,
    })
}

/// The `frameworks` table of a configuration file.
#[derive(Default)]
struct Frameworks {
    /// Whether the rules of each framework apply, for the frameworks that the table sets.
    enabled: FxHashMap<Framework, bool>,
    /// The configuration files of Sphinx, if they are set.
    sphinx_configuration: Option<Vec<PathBuf>>,
    /// The settings modules of Django that are given.
    django_settings: Vec<PathBuf>,
}

/// Get the `frameworks` table, which maps the name of a framework to whether its rules apply. Sphinx
/// and Django can instead have a table of their settings, which turns their rules on.
fn parse_frameworks(options: &Table, place: &str) -> Result<Frameworks, String> {
    let mut frameworks = Frameworks::default();
    let table = match options.get("frameworks") {
        None => return Ok(frameworks),
        Some(Value::Table(table)) => table,
        Some(_) => return Err(format!("Option 'frameworks' in {place} must be a table")),
    };
    for (name, value) in table {
        let Some(framework) = Framework::from_name(name) else {
            let known: Vec<String> = Framework::ALL
                .iter()
                .map(|framework| format!("'{}'", framework.name()))
                .collect();
            return Err(format!(
                "Option 'frameworks' in {place} has an unknown framework '{name}', expected one of {}",
                known.join(", ")
            ));
        };
        let has_table = matches!(framework, Framework::Sphinx | Framework::Django);
        match value {
            Value::Boolean(value) => {
                frameworks.enabled.insert(framework, *value);
            }
            Value::Table(settings) if framework == Framework::Sphinx => {
                frameworks.enabled.insert(framework, true);
                frameworks.sphinx_configuration = parse_sphinx(settings, place)?;
            }
            Value::Table(settings) if framework == Framework::Django => {
                frameworks.enabled.insert(framework, true);
                frameworks.django_settings = parse_django(settings, place)?;
            }
            _ if has_table => {
                return Err(format!(
                    "Option 'frameworks.{name}' in {place} must be a boolean or a table"
                ));
            }
            _ => {
                return Err(format!(
                    "Option 'frameworks.{name}' in {place} must be a boolean"
                ));
            }
        }
    }
    Ok(frameworks)
}

/// Get the `configuration` option of the `frameworks.sphinx` table: the configuration files of Sphinx,
/// or None if it is not set.
fn parse_sphinx(table: &Table, place: &str) -> Result<Option<Vec<PathBuf>>, String> {
    let place = format!("the 'frameworks.sphinx' table of {place}");
    check_names(table, &["configuration"], &place)?;
    if !table.contains_key("configuration") {
        return Ok(None);
    }
    Ok(Some(
        string_list(table, "configuration", &place)?
            .into_iter()
            .map(PathBuf::from)
            .collect(),
    ))
}

/// Get the `settings` option of the `frameworks.django` table: the settings modules of Django, by
/// their files.
fn parse_django(table: &Table, place: &str) -> Result<Vec<PathBuf>, String> {
    let place = format!("the 'frameworks.django' table of {place}");
    check_names(table, &["settings"], &place)?;
    Ok(string_list(table, "settings", &place)?
        .into_iter()
        .map(PathBuf::from)
        .collect())
}

/// Check that every option is a known one.
fn check_names(options: &Table, known: &[&str], place: &str) -> Result<(), String> {
    match options.keys().find(|name| !known.contains(&name.as_str())) {
        Some(name) => Err(format!("Unknown option '{name}' in {place}")),
        None => Ok(()),
    }
}

/// Get an option that lists decorators, as dotted names without the `@`.
fn decorators(options: &Table, name: &str, place: &str) -> Result<Vec<String>, String> {
    let decorators = string_list(options, name, place)?;
    if let Some(decorator) = decorators
        .iter()
        .find(|decorator| decorator.starts_with('@'))
    {
        return Err(format!(
            "Option '{name}' in {place} must list decorators without the '@': '{decorator}'"
        ));
    }
    Ok(decorators)
}

/// Get an option that is a string, or None if it is not set.
fn string_option<'a>(
    options: &'a Table,
    name: &str,
    place: &str,
) -> Result<Option<&'a str>, String> {
    match options.get(name) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(format!("Option '{name}' in {place} must be a string")),
    }
}

/// Get an option that is a boolean, or None if it is not set.
fn bool_option(options: &Table, name: &str, place: &str) -> Result<Option<bool>, String> {
    match options.get(name) {
        None => Ok(None),
        Some(Value::Boolean(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("Option '{name}' in {place} must be a boolean")),
    }
}

/// Get an option that is a list of strings, empty if the option is not set.
fn string_list(options: &Table, name: &str, place: &str) -> Result<Vec<String>, String> {
    let invalid = || format!("Option '{name}' in {place} must be a list of strings");
    match options.get(name) {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(item) => Ok(item.clone()),
                _ => Err(invalid()),
            })
            .collect(),
        Some(_) => Err(invalid()),
    }
}
