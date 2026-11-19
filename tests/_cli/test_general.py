import os
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
    "read",
    [
        "print(source.value())\n",
        "def show(): print(source.value())\nshow()\n",
    ],
    ids=["module", "function"],
)
def test_import_aliases_in_main_guard(tmp_path: Path, read: str) -> None:
    (tmp_path / "entry.py").write_text(
        'if __name__ == "__main__":\n    import direct as source\nelse:\n    import imported as source\n' + read
    )
    (tmp_path / "direct.py").write_text("def value(): return 8\n")
    (tmp_path / "imported.py").write_text("def value(): return 9\n")
    (tmp_path / "inspect_entry.py").write_text("import entry\n")

    result = run([], cwd=tmp_path)

    assert result.exit_code == 0
    assert result.stdout == ""


def test_read_alias_inside_main_guard_branch(tmp_path: Path) -> None:
    (tmp_path / "entry.py").write_text(
        'if "__main__" == __name__:\n'
        "    import direct as source\n"
        "    print(source.script_value())\n"
        "else:\n"
        "    import imported as source\n"
        "    def show(): print(source.import_value())\n"
        "    show()\n"
    )
    (tmp_path / "direct.py").write_text("def script_value(): return 8\ndef import_value(): return -1\n")
    (tmp_path / "imported.py").write_text("def script_value(): return -1\ndef import_value(): return 9\n")
    (tmp_path / "inspect_entry.py").write_text("import entry\n")

    result = run([], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == (
        "direct.py:2:5: unused-function 'import_value'\nimported.py:1:5: unused-function 'script_value'\n"
    )


def test_read_before_main_guard_uses_earlier_import(tmp_path: Path) -> None:
    (tmp_path / "entry.py").write_text(
        "import earlier as source\n"
        "print(source.value())\n"
        'if __name__ == "__main__":\n'
        "    import later as source\n"
        "else:\n"
        "    import another as source\n"
    )
    for name in ("earlier", "later", "another"):
        (tmp_path / f"{name}.py").write_text("def value(): return 7\n")
    (tmp_path / "inspect_entry.py").write_text("import entry\n")

    result = run([], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == "another.py:1:5: unused-function 'value'\nlater.py:1:5: unused-function 'value'\n"


@pytest.mark.parametrize(
    "read", ["source.value()\n", "def show(): source.value()\nshow()\n"], ids=["module", "function"]
)
def test_import_after_main_guard_replaces_both_aliases(tmp_path: Path, read: str) -> None:
    (tmp_path / "entry.py").write_text(
        "import shared as source\n"
        'if __name__ == "__main__":\n'
        "    import direct as source\n"
        "else:\n"
        "    import imported as source\n"
        "import shared as source\n" + read
    )
    for name in ("direct", "imported", "shared"):
        (tmp_path / f"{name}.py").write_text("def value(): return 7\n")
    (tmp_path / "inspect_entry.py").write_text("import entry\n")

    result = run([], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == "direct.py:1:5: unused-function 'value'\nimported.py:1:5: unused-function 'value'\n"


def test_conditional_import_in_main_guard_keeps_fallback(tmp_path: Path) -> None:
    (tmp_path / "entry.py").write_text(
        "import sys\n"
        "import shared as source\n"
        'if __name__ == "__main__":\n'
        "    if len(sys.argv) > 1:\n"
        "        import direct as source\n"
        "        source.branch()\n"
        "else:\n"
        "    import imported as source\n"
        "source.value()\n"
    )
    for name in ("direct", "imported", "shared"):
        (tmp_path / f"{name}.py").write_text("def value(): return 7\ndef branch(): pass\n")
    (tmp_path / "inspect_entry.py").write_text("import entry\n")

    result = run([], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == "imported.py:2:5: unused-function 'branch'\nshared.py:2:5: unused-function 'branch'\n"


@pytest.mark.parametrize("branch", ["script", "imported", "shared"])
def test_base_of_class_in_main_guard(tmp_path: Path, branch: str) -> None:
    subclass = "class Derived(Base): pass\nDerived().execute()\n"
    inside_branch = "    class Derived(Base): pass\n    Derived().execute()\n"
    (tmp_path / "entry.py").write_text(
        'if __name__ == "__main__":\n'
        "    from script_classes import Base\n"
        + (inside_branch if branch == "script" else "")
        + "else:\n    from imported_classes import Base\n"
        + (inside_branch if branch == "imported" else "")
        + (subclass if branch == "shared" else "")
    )
    for name in ("script_classes", "imported_classes"):
        (tmp_path / f"{name}.py").write_text("class Base:\n    def execute(self): return 7\n")
    (tmp_path / "inspect_entry.py").write_text("import entry\n")

    result = run([], cwd=tmp_path)

    expected = {
        "script": "imported_classes.py:2:9: unused-method 'execute'\n",
        "imported": "script_classes.py:2:9: unused-method 'execute'\n",
        "shared": "",
    }
    assert result.exit_code == (0 if branch == "shared" else 1)
    assert result.stdout == expected[branch]


@pytest.mark.skipif(os.name != "posix", reason="requires POSIX directory permissions")
@pytest.mark.parametrize(
    ("args", "exit_code"),
    [
        (["."], 1),
        (["private"], 1),
        ([".", "private"], 1),
        (["--exit-zero", "."], 0),
        (["--show-files", "."], 1),
        (["--show-files", "--exit-zero", "."], 0),
    ],
)
def test_unreadable_directory(tmp_path: Path, args: list[str], exit_code: int) -> None:
    private = tmp_path / "private"
    private.mkdir()
    (private / "hidden.py").write_text("def hidden(): pass\n")
    (tmp_path / "visible.py").write_text("def visible(): pass\n")
    private.chmod(0)
    try:
        try:
            next(private.iterdir(), None)
        except PermissionError as error:
            diagnostic = f"private:1:1: io-error '{error.strerror} (os error {error.errno})'\n"
        else:
            pytest.skip("the current process can enumerate the directory")

        result = run(args, cwd=tmp_path)
    finally:
        private.chmod(0o700)

    assert result.exit_code == exit_code
    if "--show-files" in args:
        assert result.stdout == "visible.py\n"
        assert result.stderr == diagnostic
    else:
        visible_finding = "visible.py:1:5: unused-function 'visible'\n" if "." in args else ""
        assert result.stdout == diagnostic + visible_finding
        assert result.stderr == ""


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
