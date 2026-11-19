import shutil
import warnings
from pathlib import Path

import pytest

from tests._cli._files import CASES_DIR, write_config
from tests._unref import run


def test_findings() -> None:
    case = CASES_DIR / "method"

    result = run([str(case)])

    assert result.exit_code == 1
    # The methods of the unused class are transitive, and not shown by default.
    assert result.stdout == f"{case}/main.py:1:7: unused-class 'Class'\n"


@pytest.mark.parametrize("option", ["--exit-zero", "-e"])
def test_exit_zero(option: str) -> None:
    case = CASES_DIR / "function"

    result = run([option, str(case)])

    # The findings are still printed.
    assert result.exit_code == 0
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_exit_zero_error() -> None:
    # An error still exits with its own status.
    result = run(["--exit-zero", "--jobs", "0", str(CASES_DIR / "function")])

    assert result.exit_code == 2


def test_file_path() -> None:
    # Only the given file is checked, not the other files next to it.
    case = CASES_DIR / "package" / "pkg"

    result = run([str(case / "module.py")])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/module.py:1:5: unused-function 'function'\n"


def test_default_path(tmp_path: Path) -> None:
    # Without arguments and without a `pyproject.toml`, the current directory is checked.
    project = tmp_path / "project"
    shutil.copytree(CASES_DIR / "package", project)

    result = run([], cwd=project)

    assert result.exit_code == 1
    assert (
        result.stdout == "pkg/module.py:1:5: unused-function 'function'\npkg/sub/module.py:1:7: unused-class 'Class'\n"
    )


@pytest.mark.parametrize(
    ("script_import", "worker"),
    [
        (
            "from pkg.lib.worker import execute\n",
            "def execute():\n    from tasks import value\n    return value()\n",
        ),
        (
            "import pkg.lib.worker\n",
            "from tasks import value\nprint(value())\ndef execute(): pass\n",
        ),
    ],
)
def test_fully_qualified_import_keeps_script_directory(tmp_path: Path, script_import: str, worker: str) -> None:
    (tmp_path / "pkg/lib").mkdir(parents=True)
    (tmp_path / "pkg/__init__.py").touch()
    (tmp_path / "pkg/lib/__init__.py").touch()
    entry = "execute()" if script_import.startswith("from") else "pass"
    files = {
        "pkg/run.py": f'{script_import}if __name__ == "__main__": {entry}\n',
        "pkg/lib/worker.py": worker,
        "pkg/lib/tasks.py": "def value(): return -1\n",
        "pkg/tasks.py": "def value(): return 7\n",
    }
    for name, contents in files.items():
        (tmp_path / name).write_text(contents)

    result = run(["."], cwd=tmp_path, env={"PYTHONPATH": str(tmp_path)})

    expected = f"{Path('pkg/lib/tasks.py')}:1:5: unused-function 'value'\n"
    if script_import.startswith("import"):
        expected += f"{Path('pkg/lib/worker.py')}:3:5: unused-function 'execute'\n"
    assert result.exit_code == 1
    assert result.stdout == expected
    assert result.stderr == ""


@pytest.mark.parametrize(
    ("files", "expected"),
    [
        pytest.param(
            {
                "pkg/lib/worker.py": "from .helper import work\ndef execute(): return work()\n",
                "pkg/lib/helper.py": "from tasks import live\ndef work(): return live()\n",
                "pkg/tasks.py": "def live(): return 7\n",
            },
            "",
            id="unique-relative-helper",
        ),
        pytest.param(
            {
                "pkg/lib.py": "from tasks import dormant\nprint(dormant())\n",
                "pkg/lib/worker.py": "from tasks import live\ndef execute(): return live()\n",
                "pkg/tasks.py": "def live(): return 7\ndef dormant(): return -1\n",
            },
            f"{Path('pkg/tasks.py')}:2:5: unused-function 'dormant'\n",
            id="package-shadows-module-at-dotted-prefix",
        ),
    ],
)
def test_qualified_script_imports_load_the_resolved_module(
    tmp_path: Path, files: dict[str, str], expected: str
) -> None:
    (tmp_path / "pkg/lib").mkdir(parents=True)
    (tmp_path / "pkg/__init__.py").touch()
    (tmp_path / "pkg/lib/__init__.py").touch()
    (tmp_path / "pkg/run.py").write_text(
        'from pkg.lib.worker import execute\nif __name__ == "__main__": print(execute())\n'
    )
    for name, contents in files.items():
        (tmp_path / name).write_text(contents)

    result = run(["."], cwd=tmp_path, env={"PYTHONPATH": str(tmp_path)})

    assert result.stdout == expected
    assert result.exit_code == int(bool(expected))
    assert result.stderr == ""


@pytest.mark.parametrize("case", ["dormant", "competing-module", "competing-function"])
def test_qualified_import_does_not_assume_a_script_directory(tmp_path: Path, case: str) -> None:
    (tmp_path / "pkg/lib").mkdir(parents=True)
    (tmp_path / "pkg/__init__.py").touch()
    (tmp_path / "pkg/lib/__init__.py").touch()
    files = {
        "pkg/lib/tasks.py": "def value(): return -1\n",
        "pkg/tasks.py": "def value(): return 7\n",
    }
    if case == "dormant":
        files["pkg/run.py"] = 'def never():\n    import pkg.lib.worker\nif __name__ == "__main__": pass\n'
        files["pkg/lib/worker.py"] = "from tasks import value\nprint(value())\n"
    elif case == "competing-module":
        files["pkg/run.py"] = 'from pkg.lib.worker import execute\nif __name__ == "__main__": print(execute())\n'
        files["pkg/lib/run.py"] = 'import pkg.lib.worker\nif __name__ == "__main__": print(9)\n'
        files["pkg/lib/worker.py"] = "def execute():\n    from tasks import value\n    return value()\n"
    else:
        files["pkg/run.py"] = 'import pkg.lib.worker\nif __name__ == "__main__": print(9)\n'
        files["pkg/lib/run.py"] = (
            'def main():\n    from pkg.lib.worker import execute\n    print(execute())\nif __name__ == "__main__": main()\n'
        )
        files["pkg/lib/worker.py"] = "def execute():\n    from tasks import value\n    return value()\n"
    for name, contents in files.items():
        (tmp_path / name).write_text(contents)

    result = run(["."], cwd=tmp_path, env={"PYTHONPATH": str(tmp_path)})

    expected = f"{Path('pkg/lib/tasks.py')}:1:5: unused-function 'value'\n"
    if case == "dormant":
        expected += f"{Path('pkg/run.py')}:1:5: unused-function 'never'\n"
    expected += f"{Path('pkg/tasks.py')}:1:5: unused-function 'value'\n"
    assert result.exit_code == 1
    assert result.stdout == expected
    assert result.stderr == ""


@pytest.mark.parametrize(
    ("binding", "call"),
    [
        ("from .worker import execute", "execute()"),
        ("from . import worker", "worker.execute()"),
        ("from pkg.lib import worker", "worker.execute()"),
    ],
)
def test_competing_import_forms_do_not_assign_script_directory(tmp_path: Path, binding: str, call: str) -> None:
    (tmp_path / "pkg/lib").mkdir(parents=True)
    (tmp_path / "pkg/__init__.py").touch()
    (tmp_path / "pkg/lib/__init__.py").touch()
    files = {
        "pkg/run.py": 'import pkg.lib.worker\nif __name__ == "__main__": print(7)\n',
        "pkg/lib/run.py": 'from pkg.lib.wrapper import launch\nif __name__ == "__main__": print(launch())\n',
        "pkg/lib/wrapper.py": f"{binding}\ndef launch(): return {call}\n",
        "pkg/lib/worker.py": "def execute():\n    from tasks import value\n    return value()\n",
        "pkg/lib/tasks.py": "def value(): return 9\n",
        "pkg/tasks.py": "def value(): return -1\n",
    }
    for name, contents in files.items():
        (tmp_path / name).write_text(contents)

    result = run(["."], cwd=tmp_path, env={"PYTHONPATH": str(tmp_path)})

    assert result.exit_code == 1
    assert result.stdout == (
        f"{Path('pkg/lib/tasks.py')}:1:5: unused-function 'value'\n"
        f"{Path('pkg/tasks.py')}:1:5: unused-function 'value'\n"
    )


def test_gitignored_given_directory() -> None:
    # A directory that git ignores is still checked when it is given directly, as with ruff and ty.
    result = run(["main.py", "generated"], cwd=CASES_DIR / "gitignore")

    # `used_by_ignored` is used by the ignored file.
    assert result.exit_code == 1
    assert (
        result.stdout == "main.py:5:5: unused-function 'used_by_kept'\nmain.py:9:5: unused-function 'used_by_nested'\n"
    )


def test_gitignored_given_directory_parent(tmp_path: Path) -> None:
    # A directory under one that git ignores is checked when it is given directly.
    (tmp_path / ".git").mkdir()
    (tmp_path / ".gitignore").write_text("/ignored\n")
    shutil.copytree(CASES_DIR / "function", tmp_path / "ignored" / "function")

    result = run([str(tmp_path / "ignored" / "function")])

    assert result.exit_code == 1
    assert result.stdout == f"{tmp_path}/ignored/function/main.py:1:5: unused-function 'function'\n"


def test_default_excluded_given_directory() -> None:
    # A directory that is excluded by default is still checked when it is given directly.
    result = run(["main.py", "node_modules"], cwd=CASES_DIR / "default_excluded")

    assert result.exit_code == 0
    assert result.stdout == ""


def test_syntax_error() -> None:
    # A file that does not parse is reported, and the rest of the project is still checked.
    case = CASES_DIR / "syntax_error"

    result = run([str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/bad.py:7:7: invalid-syntax 'Simple statements must be separated by newlines or semicolons'\n"
        f"{case}/good.py:1:5: unused-function 'unused'\n"
    )


@pytest.mark.parametrize(
    ("name", "error"),
    [
        ("undecodable.txt", "(unicode error) 'euc-jp' codec can't decode the source"),
        ("unknown_encoding.txt", "unknown encoding: unknown-codec"),
    ],
)
def test_undecodable(name: str, error: str) -> None:
    # A file that does not decode with the encoding it declares, or that declares an encoding that does
    # not exist, is a syntax error, as in Python. The files are not named `.py`, so that the cases, which
    # read every `.py` file, do not read them, and a file that is given directly is checked whatever its
    # name.
    path = CASES_DIR / "syntax_error" / name

    result = run([str(path)])

    assert result.exit_code == 1
    assert result.stdout == f"{path}:1:1: invalid-syntax '{error}'\n"


@pytest.mark.parametrize("name", ["missing.py", "missing", "missing/module.py", "missing/../main.py"])
def test_missing_path(name: str) -> None:
    # A given path that does not exist, whether a file or a directory, is reported as a file that
    # cannot be read, as with ruff, and the rest of the project is still checked. A path through a
    # directory that does not exist does not exist either, even when a `..` leads back out of it.
    case = CASES_DIR / "method"
    missing = case / name

    result = run([str(missing), str(case)])

    assert result.exit_code == 1
    assert result.stdout == (
        f"{case}/main.py:1:7: unused-class 'Class'\n{missing}:1:1: io-error 'No such file or directory (os error 2)'\n"
    )


def test_syntax_warning() -> None:
    # The warnings of parsing, such as for an invalid escape sequence, are not shown. Pytest records the
    # warnings itself, so they are recorded here to see them.
    case = CASES_DIR / "syntax_warning"

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        result = run([str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:1: unused-variable 'PATTERN'\n"
    assert result.stderr == ""
    assert caught == []


def test_jobs_short_option() -> None:
    # Every run of a case passes `--jobs`, so only the short option is left to check.
    case = CASES_DIR / "function"

    result = run(["-j", "1", str(case)])

    assert result.exit_code == 1
    assert result.stdout == f"{case}/main.py:1:5: unused-function 'function'\n"


def test_invalid_jobs() -> None:
    result = run(["--jobs", "0", str(CASES_DIR / "function")])

    assert result.exit_code == 2
    assert result.stdout == ""
    # The message is colored in some terminals, so only a part of it that has no color is checked.
    assert "the number of jobs must be at least 1" in result.stderr


def test_several_threads(tmp_path: Path) -> None:
    # The files are analyzed by several threads at once, which finds the same as one thread.
    for index in range(4):
        shutil.copytree(CASES_DIR, tmp_path / f"copy_{index}")

    one = run(["--jobs", "1", "--show-transitive", str(tmp_path)])
    several = run(["--jobs", "4", "--show-transitive", str(tmp_path)])

    assert one.exit_code == 1
    assert one.stdout
    assert several.stdout == one.stdout


@pytest.mark.parametrize(
    ("ignore_file", "config"),
    [(".gitignore", "unref.toml"), ("unused.gitignore", "pyproject.toml")],
)
def test_pattern_through_given_directory(tmp_path: Path, ignore_file: str, config: str) -> None:
    # A pattern that matches what is under a directory given directly, only through the directory, does
    # not count, whether git ignores it or it is excluded.
    (tmp_path / ".git").mkdir()
    (tmp_path / ignore_file).write_text("generated/**\n")
    write_config(tmp_path, '[tool.unref]\nexclude = ["generated/**"]\n', config)
    (tmp_path / "generated").mkdir()
    (tmp_path / "generated" / "module.py").write_text("def function():\n    pass\n")

    given = run(["generated"], cwd=tmp_path)
    whole = run([], cwd=tmp_path)

    assert given.stdout == "generated/module.py:1:5: unused-function 'function'\n"
    assert whole.stdout == ""
