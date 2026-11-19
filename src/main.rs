//! An extremely fast Python dead code finder, written in Rust.

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
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use crate::config::{OutputFormat, load_config};
use crate::discover::{discover_for_listing, rebase};
use crate::engine::find_unused;
use crate::external::interpreter_environment;
use crate::models::Finding;
use crate::modules::resolve;

// The check makes many small allocations, which these serve faster than the allocator of the system.
#[cfg(not(all(target_os = "linux", target_arch = "aarch64", target_env = "gnu")))]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "gnu"))]
#[global_allocator]
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// The stack of the thread that runs the check. Following base classes and imports recurses, so it is
/// larger than the default.
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// An extremely fast Python dead code finder, written in Rust.
///
/// Exits with a non-zero status if anything is found.
#[derive(Parser)]
#[command(name = "unref", version, about)]
struct Cli {
    /// Files and directories to check. Defaults to the root of the project.
    paths: Vec<PathBuf>,

    /// Configuration file. Defaults to the closest pyproject.toml, from the current directory up.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Number of threads. Defaults to one for each CPU.
    #[arg(long, short, value_parser = parse_jobs)]
    jobs: Option<NonZeroUsize>,

    /// Python environment or interpreter of the project. Defaults to the active one, or else the .venv of the project root, or else the one that unref is installed in.
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

    /// Exit with status 0, even if something is found.
    #[arg(long, short)]
    exit_zero: bool,

    /// Print the files that would be checked, and exit.
    #[arg(long)]
    show_files: bool,

    /// Files and directories to leave out, in place of the `exclude` option. Relative to the current
    /// directory: `gen` is only `./gen`, `**/gen` is one at any depth.
    #[arg(
        long,
        value_name = "FILE_PATTERN",
        value_delimiter = ',',
        help_heading = "File selection"
    )]
    exclude: Option<Vec<String>>,

    /// Files and directories to leave out, on top of the `exclude` option. Relative to the current
    /// directory: `gen` is only `./gen`, `**/gen` is one at any depth.
    #[arg(
        long,
        value_name = "FILE_PATTERN",
        value_delimiter = ',',
        help_heading = "File selection"
    )]
    extend_exclude: Vec<String>,

    /// Leave out the files and directories that are excluded or ignored even when they are given
    /// directly.
    #[arg(
        long,
        overrides_with = "no_force_exclude",
        help_heading = "File selection"
    )]
    force_exclude: bool,

    /// Check the files and directories that are given directly even when they are excluded or ignored.
    #[arg(
        long,
        overrides_with = "force_exclude",
        help_heading = "File selection"
    )]
    no_force_exclude: bool,

    /// Leave out the files and directories that git ignores.
    #[arg(
        long,
        overrides_with = "no_respect_gitignore",
        help_heading = "File selection"
    )]
    respect_gitignore: bool,

    /// Check the files and directories that git ignores too.
    #[arg(
        long,
        overrides_with = "respect_gitignore",
        help_heading = "File selection"
    )]
    no_respect_gitignore: bool,
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
    let mut options = match load_config(cli.config.as_deref()) {
        Ok(options) => options,
        Err(error) => return fail(&error),
    };

    // The patterns of the command line are relative to the current directory, which is under the root
    // of the project, and those of the configuration to the root.
    let directory = directory_under_root(&options.root);
    if let Some(exclude) = &cli.exclude {
        options.exclude = exclude
            .iter()
            .map(|pattern| rebase(pattern, &directory, &options.root))
            .collect();
    }
    options.exclude.extend(
        cli.extend_exclude
            .iter()
            .map(|pattern| rebase(pattern, &directory, &options.root)),
    );
    options.force_exclude = flag(
        cli.force_exclude,
        cli.no_force_exclude,
        options.force_exclude,
    );
    options.respect_gitignore = flag(
        cli.respect_gitignore,
        cli.no_respect_gitignore,
        options.respect_gitignore,
    );
    let paths = if cli.paths.is_empty() {
        vec![options.root.clone()]
    } else {
        cli.paths
    };

    if cli.show_files {
        let (files, errors) = discover_for_listing(&paths, &options);
        let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
        for file in &files {
            if writeln!(stdout, "{}", file.display()).is_err() {
                break;
            }
        }
        let _ = stdout.flush();
        let mut stderr = std::io::stderr().lock();
        for error in &errors {
            let _ = writeln!(stderr, "{error}");
        }
        return if errors.is_empty() || cli.exit_zero {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let environment = match cli.python.or_else(|| options.python.clone()) {
        None => find_environment(&options.root),
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

    options.python = environment;
    let findings = pool.install(|| find_unused(&options, &paths));

    let output_format = cli.output_format.unwrap_or(options.output_format);
    let show_transitive = flag(
        cli.show_transitive,
        cli.no_show_transitive,
        options.show_transitive,
    );
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
    if findings.is_empty() || cli.exit_zero {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The value of an option that the command line can turn on or off, or else the one of the
/// configuration.
fn flag(on: bool, off: bool, configured: bool) -> bool {
    if on {
        true
    } else if off {
        false
    } else {
        configured
    }
}

/// The current directory relative to the root of the project, which is the current directory or one of
/// the directories above it, such as `pkg` for the root `..`.
fn directory_under_root(root: &Path) -> PathBuf {
    let depth = root
        .components()
        .filter(|component| *component == Component::ParentDir)
        .count();
    if depth == 0 {
        return PathBuf::new();
    }
    let Ok(current) = std::env::current_dir() else {
        return PathBuf::new();
    };
    let names: Vec<_> = current.components().collect();
    names[names.len().saturating_sub(depth)..].iter().collect()
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
/// root of the project, or else the environment that the command is installed in, if it is one.
///
/// A hook of prek or pre-commit runs in an environment of its own, which it activates, so an active
/// environment that holds the command itself is not taken first.
fn find_environment(root: &Path) -> Option<PathBuf> {
    let own = own_environment();
    if let Some(active) = std::env::var_os("VIRTUAL_ENV").filter(|active| !active.is_empty()) {
        let active = PathBuf::from(active);
        if Some(resolve(&active)) != own {
            return Some(active);
        }
    }
    let venv = if root == Path::new(".") {
        PathBuf::from(".venv")
    } else {
        root.join(".venv")
    };
    if venv.is_dir() {
        return Some(venv);
    }
    own.filter(|environment| environment.join("pyvenv.cfg").is_file())
}

/// The environment that the command is installed in, the one above its `bin` or `Scripts` directory.
fn own_environment() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    Some(resolve(&interpreter_environment(&executable)))
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
