from pathlib import Path
import subprocess
import sys
from .conftest import ProjectDir

import pytest

@pytest.fixture
def project_dir(tmp_path) -> ProjectDir:
    subprocess.check_call(
        [sys.executable, "-m", "uv", "venv", "--seed", ".venv", "-q"],
        cwd=tmp_path,
    )
    return ProjectDir(tmp_path, tmp_path / ".venv")

def test_simple(project_dir: ProjectDir):
    project_dir.write_tree(
        {
            "pyproject.toml":
                """\
                [tool.impending]
                lockfile = "requirements.txt"
                """,
            "requirements.txt": "requests\n",
            "project/__init__.py": "__import__('impending').install()",
            "project/doom.py": "import requests",
         },
    )
    project_dir.install_impending()

    # Note that `requests` is not installed in the project's venv
    assert "requests" not in subprocess.check_output(
        [str(project_dir.python_path), "-m", "pip", "list"],
        text=True,
    )
    # Run the code that attempts to import `requests`. Note this succeeds.
    subprocess.check_call(
        [str(project_dir.python_path), "-m", "project.doom"],
        cwd=str(project_dir.path),
    )

    # Note that `requests` is now in the project's venv.
    assert "requests" in subprocess.check_output(
        [str(project_dir.python_path), "-m", "pip", "list"],
        text=True,
    )
