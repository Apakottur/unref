from tests._api.check_setup import CheckSetup


def test_empty_file(check_setup: CheckSetup) -> None:
    check_setup.assert_findings(
        files={"main.py": ""},
        findings=[],
    )
