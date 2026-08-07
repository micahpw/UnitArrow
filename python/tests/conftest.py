import pathlib
import pytest


@pytest.fixture(scope="session")
def repo_root():
    """The repository root, found by walking up for a marker.

    Tests read the shipping registry rather than an inline fixture, so a
    registry that stops loading fails the Python suite too — it is the artifact
    a user actually loads.
    """
    here = pathlib.Path(__file__).resolve()
    for parent in here.parents:
        if (parent / "registries" / "power-systems.toml").exists():
            return parent
    raise RuntimeError("repository root not found")
