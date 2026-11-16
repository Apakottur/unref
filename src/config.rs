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
    /// The options that apply only to some of the files of the project.
    pub overrides: Vec<Override>,
    /// The Python environment or interpreter of the project, or None if it is not set.
    pub python: Option<PathBuf>,
    /// The pattern of the names of the variables that are not used on purpose.
    pub dummy_variable_rgx: String,
    /// The object references of the entry points of the project, such as `package.module:function`.
    pub entry_points: Vec<String>,
    /// Whether the rules of each framework apply, for the frameworks that the file sets.
    pub frameworks: FxHashMap<Framework, bool>,
    /// How the findings are printed.
    pub output_format: OutputFormat,
    /// Whether the unused definitions that only other unused definitions reach are printed too.
    pub show_transitive: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            used_names: Vec::new(),
            used_decorators: Vec::new(),
            exclude: Vec::new(),
            overrides: Vec::new(),
            python: None,
            dummy_variable_rgx: DEFAULT_DUMMY_VARIABLE_RGX.to_string(),
            entry_points: Vec::new(),
            frameworks: FxHashMap::default(),
            output_format: OutputFormat::Concise,
            show_transitive: false,
        }
    }
}

/// Load the configuration from a TOML file.
///
/// A file named `pyproject.toml` holds the options in its `[tool.unref]` table, and the entry points
/// of the project in its `[project]` table. Any other file holds the options at the top level. Without
/// a file, the `pyproject.toml` of the current directory is read, if there is one.
///
/// Returns an error message if the file cannot be read or is not valid.
pub(crate) fn load_config(path: Option<&Path>) -> Result<Config, String> {
    let path = if let Some(path) = path {
        path.to_path_buf()
    } else {
        let path = PathBuf::from(PYPROJECT);
        if !path.is_file() {
            return Ok(Config::default());
        }
        path
    };
    let shown = path.display();

    let text = std::fs::read(&path).map_err(|error| {
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
        entry_points = parse_entry_points(&project, &path)?;
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

    let mut config = parse(&options, &path)?;
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
            "overrides",
            "python",
            "dummy-variable-rgx",
            "frameworks",
            "output-format",
            "show-transitive",
        ],
        &place,
    )?;

    let overrides = match options.get("overrides") {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Table(table) => Ok(table),
                _ => Err(format!(
                    "Option 'overrides' in {place} must be a list of tables"
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err(format!(
                "Option 'overrides' in {place} must be a list of tables"
            ));
        }
    };

    let python = match options.get("python") {
        None => None,
        Some(Value::String(python)) => Some(PathBuf::from(python)),
        Some(_) => return Err(format!("Option 'python' in {place} must be a string")),
    };

    let dummy_variable_rgx = match options.get("dummy-variable-rgx") {
        None => DEFAULT_DUMMY_VARIABLE_RGX.to_string(),
        Some(Value::String(pattern)) => pattern.clone(),
        Some(_) => {
            return Err(format!(
                "Option 'dummy-variable-rgx' in {place} must be a string"
            ));
        }
    };
    if let Err(error) = fancy_regex::Regex::new(&dummy_variable_rgx) {
        return Err(format!(
            "Option 'dummy-variable-rgx' in {place} is not a valid regular expression: {error}"
        ));
    }

    let output_format = match options.get("output-format") {
        None => OutputFormat::Concise,
        Some(Value::String(format)) if format == "concise" => OutputFormat::Concise,
        Some(Value::String(format)) if format == "github" => OutputFormat::Github,
        Some(_) => {
            return Err(format!(
                "Option 'output-format' in {place} must be one of 'concise', 'github'"
            ));
        }
    };

    let show_transitive = match options.get("show-transitive") {
        None => false,
        Some(Value::Boolean(value)) => *value,
        Some(_) => {
            return Err(format!(
                "Option 'show-transitive' in {place} must be a boolean"
            ));
        }
    };

    Ok(Config {
        used_names: string_list(options, "used-names", &place)?,
        used_decorators: decorators(options, "used-decorators", &place)?,
        // As with ruff, the paths are relative to the current directory, not to the configuration file.
        exclude: string_list(options, "exclude", &place)?,
        overrides: overrides
            .into_iter()
            .map(|table| parse_override(table, path))
            .collect::<Result<_, _>>()?,
        python,
        dummy_variable_rgx,
        entry_points: Vec::new(),
        frameworks: parse_frameworks(options, &place)?,
        output_format,
        show_transitive,
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
        // As with ruff, the paths are relative to the current directory, not to the configuration file.
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

/// Get the `frameworks` table, which maps the name of a framework to whether its rules apply.
fn parse_frameworks(options: &Table, place: &str) -> Result<FxHashMap<Framework, bool>, String> {
    let mut frameworks = FxHashMap::default();
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
        let Value::Boolean(value) = value else {
            return Err(format!(
                "Option 'frameworks.{name}' in {place} must be a boolean"
            ));
        };
        frameworks.insert(framework, *value);
    }
    Ok(frameworks)
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
