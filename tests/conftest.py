from pathlib import Path

import pytest

from tests._api.check_setup import CheckSetup


@pytest.fixture
def check_setup(tmp_path: Path) -> CheckSetup:
    """
    Main fixture for testing `check`.
    """
    return CheckSetup(root=tmp_path)
