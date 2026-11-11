import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

_PYPROJECT = "pyproject.toml"


class UnrefConfigError(Exception):
    """
    The configuration file cannot be read or is not valid.
    """


@dataclass(frozen=True)
class Config:
    """
    The configuration of a check.

    Attributes:
        used_decorators: The decorators that make the definitions they decorate count as used.
        exclude: The files and directories that are not part of the project.
    """

    used_decorators: tuple[str, ...] = ()
    exclude: tuple[Path, ...] = ()


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
    for name in options:
        if name not in {"used-decorators", "exclude"}:
            raise UnrefConfigError(f"Unknown option {name!r} in the configuration file {path}")

    used_decorators = _string_list(options, "used-decorators", path)
    for decorator in used_decorators:
        if decorator.startswith("@"):
            raise UnrefConfigError(
                f"Option 'used-decorators' in the configuration file {path} "
                f"must list decorators without the '@': {decorator!r}"
            )

    return Config(
        used_decorators=tuple(used_decorators),
        # The excluded paths are relative to the directory of the configuration file.
        exclude=tuple(path.parent / item for item in _string_list(options, "exclude", path)),
    )


def _string_list(options: dict[str, Any], name: str, path: Path) -> list[str]:
    """
    Get an option that is a list of strings, empty if the option is not set.
    """
    value = options.get(name, [])
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise UnrefConfigError(f"Option {name!r} in the configuration file {path} must be a list of strings")
    return value
