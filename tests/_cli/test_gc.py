import gc
from pathlib import Path

from typer.testing import CliRunner

from unref._cli import app

_runner = CliRunner()

_CASE = Path(__file__).parent.parent / "cases" / "function"


def test_gc_stays_enabled() -> None:
    enabled = gc.isenabled()
    gc.enable()
    try:
        result = _runner.invoke(app, [str(_CASE)])

        assert result.exit_code == 1
        assert gc.isenabled()
    finally:
        if not enabled:
            gc.disable()


def test_gc_stays_disabled() -> None:
    enabled = gc.isenabled()
    gc.disable()
    try:
        result = _runner.invoke(app, [str(_CASE)])

        assert result.exit_code == 1
        assert not gc.isenabled()
    finally:
        if enabled:
            gc.enable()
