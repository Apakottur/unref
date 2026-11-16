//! Find unreferenced Python code.

mod classes;
mod config;
mod definitions;
mod discover;
mod engine;
mod external;
mod files;
mod frameworks;
mod ignores;
mod models;
mod modules;
mod positions;
mod references;
mod symbols;
mod transitive;

use std::io::Write;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use crate::config::{OutputFormat, load_config};
use crate::engine::{Options, find_unused};
use crate::models::Finding;
use crate::modules::resolve;

/// The stack of the thread that runs the check. Following base classes and imports recurses, so it is
/// larger than the default.
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// Find unreferenced Python code.
///
/// Exits with a non-zero status if anything is found.
#[derive(Parser)]
#[command(name = "unref", version, about)]
struct Cli {
    /// Files and directories to check. Defaults to the current directory.
    paths: Vec<PathBuf>,

    /// Configuration file. Defaults to ./pyproject.toml.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Number of threads. Defaults to one for each CPU.
    #[arg(long, short, value_parser = parse_jobs)]
    jobs: Option<NonZeroUsize>,

    /// Python environment or interpreter of the project. Defaults to the active one, or else ./.venv.
    #[arg(long)]
    python: Option<PathBuf>,

    /// Output format of the findings. Defaults to concise.
    #[arg(long, value_enum)]
    output_format: Option<OutputFormat>,

    /// Also print the unused definitions that only other unused definitions reach.
    #[arg(long, overrides_with = "no_show_transitive")]
    show_transitive: bool,

    /// Print only the unused definitions that are not reached from other unused definitions.
    #[arg(long, overrides_with = "show_transitive")]
    no_show_transitive: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let thread = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run(cli))
        .expect("the thread of the check starts");
    thread.join().unwrap_or(ExitCode::from(2))
}

fn run(cli: Cli) -> ExitCode {
    let options = match load_config(cli.config.as_deref()) {
        Ok(options) => options,
        Err(error) => return fail(&error),
    };

    let environment = match cli.python.or_else(|| options.python.clone()) {
        None => find_environment(),
        Some(environment) if !environment.exists() => {
            return fail(&format!(
                "The Python environment {} does not exist",
                environment.display()
            ));
        }
        Some(environment) => Some(environment),
    };

    let mut pool = rayon::ThreadPoolBuilder::new().stack_size(16 * 1024 * 1024);
    if let Some(jobs) = cli.jobs {
        pool = pool.num_threads(jobs.get());
    }
    let pool = match pool.build() {
        Ok(pool) => pool,
        Err(error) => return fail(&format!("Cannot start the threads of the check: {error}")),
    };

    let engine_options = Options {
        paths: if cli.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            cli.paths
        },
        used_names: options.used_names,
        used_decorators: options.used_decorators,
        exclude: options.exclude,
        overrides: options.overrides,
        python: environment,
        dummy_variable_rgx: options.dummy_variable_rgx,
        entry_points: options.entry_points,
        frameworks: options.frameworks,
    };
    let findings = pool.install(|| find_unused(&engine_options));

    let output_format = cli.output_format.unwrap_or(options.output_format);
    let show_transitive = if cli.show_transitive {
        true
    } else if cli.no_show_transitive {
        false
    } else {
        options.show_transitive
    };
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    for finding in &findings {
        if finding.transitive && !show_transitive {
            continue;
        }
        let line = match output_format {
            OutputFormat::Concise => finding.to_string(),
            OutputFormat::Github => github(finding),
        };
        if writeln!(stdout, "{line}").is_err() {
            break;
        }
    }
    let _ = stdout.flush();

    // A transitive finding is reached from another unused definition, so there are findings exactly
    // when some are printed.
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Parse the number of threads, which is at least 1.
fn parse_jobs(value: &str) -> Result<NonZeroUsize, String> {
    let jobs: usize = value.parse().map_err(|error| format!("{error}"))?;
    NonZeroUsize::new(jobs).ok_or_else(|| "the number of jobs must be at least 1".to_string())
}

/// Print an error and give the exit status of an error.
#[expect(clippy::print_stderr, reason = "the error is for the user")]
fn fail(message: &str) -> ExitCode {
    eprintln!("Error: {message}");
    ExitCode::from(2)
}

/// Find the virtual environment that the project runs in: the active one, or else the `.venv` of the
/// current directory, or else the environment that the command is installed in, if it is one.
///
/// A hook of prek or pre-commit runs in an environment of its own, which it activates, so an active
/// environment that holds the command itself is not taken first.
fn find_environment() -> Option<PathBuf> {
    let own = own_environment();
    if let Some(active) = std::env::var_os("VIRTUAL_ENV").filter(|active| !active.is_empty()) {
        let active = PathBuf::from(active);
        if Some(resolve(&active)) != own {
            return Some(active);
        }
    }
    let venv = Path::new(".venv");
    if venv.is_dir() {
        return Some(venv.to_path_buf());
    }
    own.filter(|environment| environment.join("pyvenv.cfg").is_file())
}

/// The environment that the command is installed in, the one above its `bin` or `Scripts` directory.
fn own_environment() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let environment = if directory
        .file_name()
        .is_some_and(|name| name == "bin" || name == "Scripts")
    {
        directory.parent()?
    } else {
        directory
    };
    Some(resolve(environment))
}

/// Format a finding as a GitHub Actions workflow command, which shows it as an annotation, laid out as
/// ruff's `github` output format lays out its diagnostics. A finding has no end, so the command has no
/// `endLine` and `endColumn`.
fn github(finding: &Finding) -> String {
    let title = escape_property(&format!("unref ({})", finding.code));
    let file = escape_property(&finding.path.display().to_string());
    format!(
        "::error title={title},file={file},line={},col={}::{}",
        finding.line,
        finding.column,
        escape_data(&finding.to_string())
    )
}

/// Escape the message of a workflow command, as the `@actions/core` toolkit of GitHub does.
fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a property of a workflow command, as the `@actions/core` toolkit of GitHub does.
fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}
