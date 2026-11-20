#!/usr/bin/env python
"""
Write `src/codecs.rs`, the encodings that Python takes in an encoding declaration, from the codecs of the Python that
runs it.

Run with `uv run scripts/generate_codecs.py`, then `cargo fmt`.
"""

import codecs
import encodings
import encodings.aliases
import importlib
import pkgutil
from pathlib import Path

_OUTPUT = Path(__file__).parent.parent / "src" / "codecs.rs"

# Codecs that map each byte to one character, without a decoding table in their module.
_SINGLE_BYTE = {"ascii", "latin_1"}

# What a byte that a codec does not decode is written as in the table, as in the tables of Python.
_UNDEFINED = "￾"


def _text_codecs() -> list[str]:
    """Get the modules of `encodings` that are text encodings, as Python looks them up by name."""
    names = []
    for module in pkgutil.iter_modules(encodings.__path__):
        try:
            info = codecs.lookup(module.name)
        except LookupError:
            continue
        if info._is_text_encoding:  # noqa: SLF001
            names.append(module.name)
    return sorted(names)


def _table(name: str) -> str | None:
    """
    Get the character of each byte in a codec that maps each byte to one character.

    The table has 128 characters, of the bytes from 0x80, when the bytes below 0x80 are ASCII, else 256.
    """
    module = importlib.import_module(f"encodings.{name}")
    if name not in _SINGLE_BYTE and not hasattr(module, "decoding_table"):
        return None
    table = "".join(bytes([byte]).decode(name, errors="replace").replace("�", _UNDEFINED) for byte in range(256))
    return table[128:] if table[:128] == "".join(map(chr, range(128))) else table


def _literal(text: str) -> str:
    """Write a string as a Rust literal, with every character that is not printable ASCII escaped."""
    escaped = "".join(
        char
        if " " <= char <= "~" and char not in '"\\'
        else f"\\u{{{ord(char):x}}}"
        if ord(char) > 0x7F
        else f"\\x{ord(char):02x}"
        for char in text
    )
    return f'"{escaped}"'


def main() -> None:
    names = _text_codecs()
    lines = [
        "//! The encodings that Python takes in an encoding declaration. Written by `scripts/generate_codecs.py`.",
        "",
        "/// A codec of Python.",
        "pub(crate) enum Codec {",
        "    /// A codec that maps each byte to one character: the characters of the bytes from 0x80 when the bytes",
        "    /// below are ASCII, else of every byte, with U+FFFE for a byte that it does not decode.",
        "    Table(&'static str),",
        "    /// Any other codec, by the name of its module.",
        "    Other(&'static str),",
        "}",
        "",
        "/// The module of the codec that an alias names, as `encodings.aliases`. The alias is normalized.",
        "pub(crate) fn alias(name: &str) -> Option<&'static str> {",
        "    Some(match name {",
    ]
    by_module: dict[str, list[str]] = {}
    for alias, module in encodings.aliases.aliases.items():
        by_module.setdefault(module, []).append(alias)
    for module, aliases in sorted(by_module.items()):
        labels = " | ".join(f'"{alias}"' for alias in sorted(aliases))
        lines.append(f'        {labels} => "{module}",')
    lines += [
        "        _ => return None,",
        "    })",
        "}",
        "",
        "/// The codec of a module of `encodings`, if it is a text encoding.",
        "pub(crate) fn codec(module: &str) -> Option<Codec> {",
        "    Some(match module {",
    ]
    # Codecs with the same table, as `latin_1` and `iso8859_1`, share an arm.
    by_codec: dict[str, list[str]] = {}
    for name in names:
        table = _table(name)
        codec = f"Codec::Table({_literal(table)})" if table is not None else f'Codec::Other("{name}")'
        by_codec.setdefault(codec, []).append(name)
    for codec, modules in by_codec.items():
        labels = " | ".join(f'"{module}"' for module in modules)
        lines.append(f"        {labels} => {codec},")
    lines += [
        "        _ => return None,",
        "    })",
        "}",
        "",
    ]
    _OUTPUT.write_text("\n".join(lines))


if __name__ == "__main__":
    main()
