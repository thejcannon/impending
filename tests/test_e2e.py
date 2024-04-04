import os
import subprocess
import sys
from .conftest import ProjectDir
from pathlib import Path
import impending

import pytest

@pytest.fixture
def project_dir(tmp_path) -> ProjectDir:
    subprocess.check_call(
        [sys.executable, "-m", "uv", "venv", ".venv", "-q"],
        cwd=tmp_path,
    )
    return ProjectDir(tmp_path, tmp_path / ".venv")

def lockfile(name):
    return Path(__file__, "..", "lockfiles", name).resolve().read_text()

@pytest.mark.parametrize(
    ["modname", "lockfile", "expected_install"],
    [
        pytest.param("direct_dep", "direct_dep", "direct_dep", id="simple"),
        pytest.param("direct_dep", "direct_dep == 0.0", "direct_dep == 0.0", id="simple_pin"),
        pytest.param("direct_dep", "direct_dep==0.0", "direct_dep==0.0", id="simple_pin"),
        pytest.param("direct_dep", "direct_dep @ http://website.com/package", "direct_dep @ http://website.com/package", id="url_pin"),
        pytest.param("direct_dep", 'direct_dep @ http://website.com/package ;python_version<"2.7"', 'direct_dep @ http://website.com/package ;python_version<"2.7"', id="url_pin"),
        pytest.param("direct_dep", "direct_dep[extra1]", "direct_dep[extra1]", id="extras"),
        pytest.param("direct_dep", "direct_dep[extra1, extra2]", "direct_dep[extra1, extra2]", id="extras"),
        pytest.param("direct_dep", 'direct_dep [security,tests] >= 2.8.1, == 2.8.* ; python_version < "2.7"', 'direct_dep [security,tests] >= 2.8.1, == 2.8.* ; python_version < "2.7"', id="complicated"),
        pytest.param("direct_dep", "direct_dep\nother_dep", "direct_dep", id="superfluous"),
        pytest.param("direct_dep", "other_dep\ndirect_dep", "direct_dep", id="superfluous"),
        pytest.param("django", lockfile("pip-compile1.txt"), "django==4.1.7\nsqlparse==0.4.3\nasgiref==3.6.0", id="requirements"),
        pytest.param("asgiref", lockfile("pip-compile1.txt"), "asgiref==3.6.0", id="requirements"),
        pytest.param("sqlparse", lockfile("pip-compile1.txt"), "sqlparse==0.4.3", id="requirements"),
        pytest.param("django", lockfile("pip-compile2.txt"), "django==4.1.7\nsqlparse==0.4.3\nasgiref==3.6.0", id="requirements"),
        pytest.param("pydantic", lockfile("pip-compile3.txt"), "pydantic==2.6.1\ntyping-extensions==4.9.0\npydantic-core==2.16.2\nannotated-types==0.6.0", id="requirements"),
        pytest.param("typing_extensions", lockfile("pip-compile3.txt"), "typing-extensions==4.9.0", id="requirements"),
        pytest.param("rich", lockfile("f2246ce.txt"), "rich", id="requirements"),
        pytest.param("keras", lockfile("f2246ce.txt"), "keras ; python_version < '3.12'", id="requirements"),
    ]
)
def test_dependency_resolution(modname: str, project_dir: ProjectDir, lockfile: str, expected_install: str):
    project_dir.install_impending()
    reqs_path = project_dir.path / "wouldve_installed.txt"
    project_dir.write_tree(
        {
            "pyproject.toml":
                f"""\
                [tool.impending]
                lockfile = "requirements.txt"
                installer_cmd = ["bash", "-c", "printf '%s\\n' \\"$@\\" > {reqs_path}", "_"]
                """,
            "requirements.txt": lockfile + "\n",
            # "project/__init__.py": "__import__('impending').install()",
            "project/doom.py": f"import {modname}",
         },
    )
    subprocess.call(
        [str(project_dir.python_path), "-m", "project.doom"],
        cwd=str(project_dir.path),
    )
    assert reqs_path.read_text() == expected_install + "\n"
