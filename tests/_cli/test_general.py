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


@pytest.fixture
def reexport_package(tmp_path: Path) -> Path:
    pkg = tmp_path / "pkg"
    wrapper = pkg / "wrapper"
    wrapper.mkdir(parents=True)
    (pkg / "__init__.py").touch()
    (pkg / "source.py").write_text("def value(): return 7\n")
    (wrapper / "source.py").write_text("def value(): return 9\n")
    return wrapper


@pytest.mark.parametrize(
    ("entry", "unused_module"),
    [
        pytest.param("from pkg.wrapper import source\nsource.value()\n", "pkg/wrapper/source.py", id="from-package"),
        pytest.param(
            "import pkg.wrapper as wrapper\nwrapper.source.value()\n", "pkg/wrapper/source.py", id="attribute"
        ),
        pytest.param("import pkg.wrapper.source as source\nsource.value()\n", "pkg/source.py", id="explicit-child"),
        pytest.param(
            "import pkg.wrapper.source\nfrom pkg.wrapper import source\nsource.value()\n",
            "pkg/source.py",
            id="explicit-child-then-from-package",
        ),
        pytest.param(
            "from pkg.wrapper.source import value\nfrom pkg.wrapper import source\nvalue(), source.value()\n",
            "pkg/source.py",
            id="explicit-from-child",
        ),
    ],
)
def test_reexport_and_same_named_child(tmp_path: Path, reexport_package: Path, entry: str, unused_module: str) -> None:
    (reexport_package / "__init__.py").write_text("from .. import source\n")
    (tmp_path / "entry.py").write_text("class Other:\n    def value(self): return -1\n\nOther()\n" + entry)

    result = run(["--show-transitive", "."], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == (
        f"entry.py:2:9: unused-method 'value'\n{Path(unused_module)}:1:5: unused-function 'value'\n"
    )
    assert result.stderr == ""


@pytest.mark.parametrize(
    ("initializer", "entry", "unused_module"),
    [
        pytest.param(
            "from . import source as child\nfrom .. import source\n",
            "from pkg.wrapper import child\nchild.value()\n",
            "pkg/source.py",
            id="child-alias-before-reexport",
        ),
        pytest.param(
            "from .source import value as child_value\nfrom .. import source\n",
            "from pkg.wrapper import child_value, source\nchild_value(), source.value()\n",
            None,
            id="child-loaded-before-reexport",
        ),
        pytest.param(
            "from .. import source\nfrom .source import value as child_value\n",
            "from pkg.wrapper import child_value, source\nchild_value(), source.value()\n",
            "pkg/source.py",
            id="child-first-loaded-after-reexport",
        ),
        pytest.param(
            "from . import source\nfrom .. import source\n",
            "from pkg.wrapper import source\nsource.value()\n",
            "pkg/wrapper/source.py",
            id="child-imported-by-name-before-reexport",
        ),
        pytest.param(
            "from . import source as child\nfrom .. import source\nchild.value()\n",
            "from pkg.wrapper import source\nsource.value()\n",
            None,
            id="child-alias-read-in-initializer",
        ),
        pytest.param(
            "if True:\n    from .. import source\nfrom . import source as exported\n",
            "from pkg.wrapper import exported\nexported.value()\n",
            "pkg/wrapper/source.py",
            id="child-name-bound-by-conditional-reexport",
        ),
    ],
)
def test_package_initializer_child_imports(
    tmp_path: Path, reexport_package: Path, initializer: str, entry: str, unused_module: str | None
) -> None:
    (reexport_package / "__init__.py").write_text(initializer)
    (tmp_path / "entry.py").write_text(entry)

    result = run(["--show-transitive", "."], cwd=tmp_path)

    assert result.exit_code == (0 if unused_module is None else 1)
    assert result.stdout == ("" if unused_module is None else f"{Path(unused_module)}:1:5: unused-function 'value'\n")
    assert result.stderr == ""


@pytest.mark.parametrize("explicit", [False, True], ids=["dormant", "explicit"])
def test_reexport_and_same_named_namespace_child(tmp_path: Path, *, explicit: bool) -> None:
    pkg = tmp_path / "pkg"
    original = pkg / "original"
    wrapper = pkg / "wrapper"
    original.mkdir(parents=True)
    (wrapper / "source").mkdir(parents=True)
    (pkg / "__init__.py").touch()
    (original / "__init__.py").write_text("from . import helpers\n")
    (original / "helpers.py").write_text("def value(): return 7\n")
    (wrapper / "__init__.py").write_text("from .. import original as source\n")
    (wrapper / "source" / "helpers.py").write_text("def value(): return 9\n")
    entry = "from pkg.wrapper import source\nsource.helpers.value()\n"
    if explicit:
        entry = "import pkg.wrapper.source.helpers\n" + entry
    (tmp_path / "entry.py").write_text(entry)

    result = run(["--show-transitive", "."], cwd=tmp_path)

    assert result.exit_code == 1
    unused = "pkg/original/helpers.py" if explicit else "pkg/wrapper/source/helpers.py"
    assert result.stdout == f"{Path(unused)}:1:5: unused-function 'value'\n"
    assert result.stderr == ""


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


def test_linked_module(tmp_path: Path) -> None:
    # A link to a file is imported by its own name. What is used through it is used in the file that it
    # links to as well, since they are one source.
    (tmp_path / "impl.py").write_text("def run():\n    return 7\n\n\ndef unused():\n    return 0\n")
    try:
        (tmp_path / "api.py").symlink_to("impl.py")
    except OSError:
        pytest.skip("the file system does not support links")
    (tmp_path / "main.py").write_text("from api import run\n\nrun()\n")

    result = run(["."], cwd=tmp_path)

    assert result.exit_code == 1
    assert result.stdout == "api.py:5:5: unused-function 'unused'\nimpl.py:5:5: unused-function 'unused'\n"


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
