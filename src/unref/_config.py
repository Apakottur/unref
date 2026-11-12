import re
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from unref._models import DEFAULT_DUMMY_VARIABLE_RGX, Override

_PYPROJECT = "pyproject.toml"

# The options of an override that select what it applies to.
_SELECTORS = ("paths", "base-classes", "class-decorators")


class UnrefConfigError(Exception):
    """
    The configuration file cannot be read or is not valid.
    """


@dataclass(frozen=True)
class Config:
    """
    The configuration of a check.

    Attributes:
        used_names: The names, as patterns, of the definitions that count as used.
        used_decorators: The decorators that make the definitions they decorate count as used.
        exclude: The patterns of the files and directories that are not part of the project.
        overrides: The options that apply only to some of the files of the project.
        python: The Python environment or interpreter of the project, or None if it is not set.
        dummy_variable_rgx: The pattern of the names of the variables that are not used on purpose.
    """

    used_names: tuple[str, ...] = ()
    used_decorators: tuple[str, ...] = ()
    exclude: tuple[str, ...] = ()
    overrides: tuple[Override, ...] = ()
    python: Path | None = None
    dummy_variable_rgx: str = DEFAULT_DUMMY_VARIABLE_RGX


def load_config(path: Path | None = None) -> Config:
    """
    Load the configuration from a TOML file.

    A file named `pyproject.toml` holds the options in its `[tool.unref]` table, any other file holds
    them at the top level.

    Args:
        path: The configuration file. Defaults to the `pyproject.toml` of the current directory, if
            there is one.

    Returns:
        The configuration.

    Raises:
        UnrefConfigError: If the file cannot be read or is not valid.
    """
    if path is None:
        path = Path(_PYPROJECT)
        if not path.is_file():
            return Config()

    try:
        with path.open("rb") as file:
            options = tomllib.load(file)
    except OSError as error:
        raise UnrefConfigError(f"Cannot read the configuration file {path}: {error.strerror}") from error
    except tomllib.TOMLDecodeError as error:
        raise UnrefConfigError(f"Invalid TOML in the configuration file {path}: {error}") from error

    if path.name == _PYPROJECT:
        options = options.get("tool", {}).get("unref", {})

    return _parse(options, path)


def _parse(options: dict[str, Any], path: Path) -> Config:
    """
    Build the configuration from the options of a configuration file.
    """
    where = f"the configuration file {path}"
    _check_names(
        options, {"used-names", "used-decorators", "exclude", "overrides", "python", "dummy-variable-rgx"}, where
    )

    overrides = options.get("overrides", [])
    if not isinstance(overrides, list) or not all(isinstance(item, dict) for item in overrides):
        raise UnrefConfigError(f"Option 'overrides' in {where} must be a list of tables")

    python = options.get("python")
    if python is not None and not isinstance(python, str):
        raise UnrefConfigError(f"Option 'python' in {where} must be a string")

    dummy_variable_rgx = options.get("dummy-variable-rgx", DEFAULT_DUMMY_VARIABLE_RGX)
    if not isinstance(dummy_variable_rgx, str):
        raise UnrefConfigError(f"Option 'dummy-variable-rgx' in {where} must be a string")
    try:
        re.compile(dummy_variable_rgx)
    except re.error as error:
        raise UnrefConfigError(
            f"Option 'dummy-variable-rgx' in {where} is not a valid regular expression: {error}"
        ) from error

    return Config(
        used_names=tuple(_string_list(options, "used-names", where)),
        used_decorators=tuple(_decorators(options, "used-decorators", where)),
        # As with ruff, the paths are relative to the current directory, not to the configuration file.
        exclude=tuple(_string_list(options, "exclude", where)),
        overrides=tuple(_parse_override(item, path) for item in overrides),
        python=None if python is None else Path(python),
        dummy_variable_rgx=dummy_variable_rgx,
    )


def _parse_override(options: dict[str, Any], path: Path) -> Override:
    """
    Build an override from one of the `overrides` tables of a configuration file.
    """
    where = f"an override in the configuration file {path}"
    _check_names(options, {"paths", "base-classes", "class-decorators", "used-names", "used-decorators"}, where)

    if options.keys().isdisjoint(_SELECTORS):
        raise UnrefConfigError(
            f"{where[0].upper()}{where[1:]} needs one of the options {', '.join(map(repr, _SELECTORS))}"
        )

    return Override(
        # As with ruff, the paths are relative to the current directory, not to the configuration file.
        paths=tuple(map(Path, _string_list(options, "paths", where))),
        base_classes=tuple(_string_list(options, "base-classes", where)),
        class_decorators=tuple(_decorators(options, "class-decorators", where)),
        used_names=tuple(_string_list(options, "used-names", where)),
        used_decorators=tuple(_decorators(options, "used-decorators", where)),
    )


def _check_names(options: dict[str, Any], known: set[str], where: str) -> None:
    """
    Check that every option is a known one.
    """
    for name in options:
        if name not in known:
            raise UnrefConfigError(f"Unknown option {name!r} in {where}")


def _decorators(options: dict[str, Any], name: str, where: str) -> list[str]:
    """
    Get an option that lists decorators, as dotted names without the `@`.
    """
    decorators = _string_list(options, name, where)
    for decorator in decorators:
        if decorator.startswith("@"):
            raise UnrefConfigError(f"Option {name!r} in {where} must list decorators without the '@': {decorator!r}")
    return decorators


def _string_list(options: dict[str, Any], name: str, where: str) -> list[str]:
    """
    Get an option that is a list of strings, empty if the option is not set.
    """
    value = options.get(name, [])
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise UnrefConfigError(f"Option {name!r} in {where} must be a list of strings")
    return value
