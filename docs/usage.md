# Usage

## Command line

Run it with [`uvx`](https://docs.astral.sh/uv/):

```shell
$ uvx unref
src/project/models.py:12:7: unused-class 'Account'
src/project/utils.py:40:5: unused-function 'parse_date'
```

`unref` checks the project that the current directory is in, or only the files and directories it is given:

```shell
uvx unref src scripts
```

`unref` exits with a non-zero status if anything is found, unless `--exit-zero` is given. See the
[rules](rules.md) for what it reports and how to ignore a finding, and the
[command line](configuration.md#command-line) for its options.

## pre-commit and prek

As a [pre-commit](https://pre-commit.com/) or [prek](https://prek.j178.dev/) hook, alongside
[Ruff](https://docs.astral.sh/ruff/), which covers the unused imports and local variables that `unref`
leaves out:

```yaml
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.17.4
    hooks:
      - id: ruff-check
      - id: ruff-format
  - repo: https://github.com/Apakottur/unref
    rev: v0.3.1
    hooks:
      - id: unref
```

Set `rev` to the [latest release](https://github.com/Apakottur/unref/releases), or let `pre-commit autoupdate`
or `prek autoupdate` fill it in.

The hook checks the whole project, not only the changed files: a definition is unused only if no file
references it.

## GitHub Actions

With `--output-format github`, the findings show as annotations on the files of a pull request:

```yaml
jobs:
  unref:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: astral-sh/setup-uv@v10
      - run: uv sync # Optional: lets unref follow base classes from the dependencies, see `python`.
      - run: uvx unref --output-format github
```
