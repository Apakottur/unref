from pathlib import Path

import unref

_CASES_DIR = Path(__file__).parent / "cases"


def test_check_transitive() -> None:
    # The library returns every finding, and tells which are transitive.
    path = _CASES_DIR / "method" / "main.py"

    findings = unref.check([path])

    assert findings == [
        unref.Finding(path=path, line=1, column=7, code="unused-class", name="Class"),
        unref.Finding(path=path, line=2, column=9, code="unused-method", name="method", transitive=True),
        unref.Finding(path=path, line=5, column=15, code="unused-method", name="async_method", transitive=True),
    ]
