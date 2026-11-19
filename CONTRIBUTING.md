# Contributing to unref

Thanks for your interest in contributing! To contribute, open a pull request with your changes, or an
issue with your bug or request.

All checks (linters, type checks, and tests) run automatically in CI through GitHub Actions.

## Development environment

`unref` is written in Rust, on [Ruff](https://github.com/astral-sh/ruff)'s parser, and ships in a Python
wheel built with [maturin](https://www.maturin.rs/), as Ruff does. Local development needs a
[Rust toolchain](https://rustup.rs/) and [uv](https://docs.astral.sh/uv/getting-started/installation/).
`rustup` installs the version of Rust that `rust-toolchain.toml` pins on the first build.

Install the development dependencies, which builds `unref` and installs it in `.venv`:

```shell
uv sync
```

`uv run` rebuilds it when its Rust source changes. To work on the Python code or the scripts without building
`unref`, install only the development dependencies with `--only-group dev`:

```shell
uv run --only-group dev mypy --config-file linters/mypy.toml .
```

## Running checks locally

Run the linters with `prek`:

```shell
uv run prek run -a
```

Run the Rust unit tests and the tests of the command, which run the `unref` installed in `.venv`:

```shell
cargo test
uv run pytest -c tests/pytest.ini tests
```

To run the tests of the command against a debug build, which builds faster, name it in `UNREF_BINARY`:

```shell
cargo build
UNREF_BINARY=target/debug/unref uv run pytest -c tests/pytest.ini tests
```

### Test cases

The behavior of the check is tested with case projects. Each directory under `tests/cases` is a small
Python project on one theme, and `tests/test_cases.py` runs the check on each of them. A finding is
expected with an `# expect:` comment on its line, and the docstring of `tests/test_cases.py` gives the
full syntax:

```python
def unused():  # expect: 5 [unused-function]
    return 1
```

To test a new behavior, add files to the case whose theme fits. Add a new directory only when the case
needs its own `pyproject.toml` or layout.

### Type checks

Run the type checks of the tests and scripts with `mypy` and `ty` (both run in CI):

```shell
uv run mypy --config-file linters/mypy.toml .
uv run ty check --project . --config-file linters/ty.toml
```

## Documentation

The documentation site lives in `docs/`, is built with [Zensical](https://zensical.org/), and is hosted on
[Read the Docs](https://unref.readthedocs.io/).

Preview it locally with live reload:

```shell
uv run --group docs zensical serve
```

Build it the way CI does (warnings, such as broken links, fail the build):

```shell
uv run --group docs zensical build --strict
```

## Releasing

To release a new version, run the interactive script (it requires the [`gh`](https://cli.github.com/) CLI,
authenticated against the repository):

```shell
uv run scripts/release.py
```
