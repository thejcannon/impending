from pathlib import Path
import subprocess
import sys
from .conftest import ProjectDir

import pytest


@pytest.fixture
def project_dir(tmp_path: Path) -> ProjectDir:
    subprocess.check_call(
        [sys.executable, "-m", "uv", "venv", "--python", sys.executable, "--seed", ".venv", "-q"],
        cwd=tmp_path,
    )
    return ProjectDir(tmp_path, tmp_path / ".venv")


def test_simple(project_dir: ProjectDir):
    project_dir.install_impending()
    project_dir.write_tree(
        {
            "pyproject.toml": """\
                [tool.impending]
                lockfile = "requirements.txt"
                install_missing_packages = true
                """,
            "requirements.txt": "requests\n",
        },
    )

    # Note that `requests` is not installed in the project's venv
    assert "requests" not in subprocess.check_output(
        [str(project_dir.python_path), "-m", "pip", "list"],
        text=True,
        cwd=str(project_dir.path),
    )
    # Run the code that attempts to import `requests`. Note this succeeds.
    subprocess.check_call(
        [str(project_dir.python_path), "-c", "import requests"],
        cwd=str(project_dir.path),
    )

    # Note that `requests` is now in the project's venv.
    assert "requests" in subprocess.check_output(
        [str(project_dir.python_path), "-m", "pip", "list"],
        text=True,
        cwd=str(project_dir.path),
    )
