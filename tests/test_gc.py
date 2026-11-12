import gc
from pathlib import Path

import unref

CASE = Path(__file__).parent / "cases" / "function"


def test_gc_stays_enabled() -> None:
    enabled = gc.isenabled()
    gc.enable()
    try:
        unref.check([CASE])
        assert gc.isenabled()
    finally:
        if not enabled:
            gc.disable()


def test_gc_stays_disabled() -> None:
    enabled = gc.isenabled()
    gc.disable()
    try:
        unref.check([CASE])
        assert not gc.isenabled()
    finally:
        if enabled:
            gc.enable()
