import subprocess
import os
import sys
from unittest import mock
from .conftest import ProjectDir as RootProjectDir
from pathlib import Path

import pytest

class ProjectDir(RootProjectDir):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.install_impending()
        self._would_installed_path = self.path / "wouldve_installed.txt"
        self._would_installed_path.touch()

    def setup(self,
        lockfile: str,
        *,
        module_map: dict[str, str] = {},
        package_companions: dict[str, list[str]] = {},
        enforce_package_versions: bool = False
    ):
        self.write_tree(
            {
                "pyproject.toml":
                    f"""\
                    [tool.impending]
                    lockfile = "requirements.txt"
                    installer_cmd = ["bash", "-c", "printf '%s\\n' \\"$@\\" >> {self._would_installed_path}", "_"]
                    module_map = {str(module_map).replace(":", "=")}
                    package_companions = {str(package_companions).replace(":", "=")}
                    install_missing_packages = true
                    enforce_package_versions = {str(enforce_package_versions).lower()}
                    """,
                "requirements.txt": lockfile + "\n",
            },
        )

    def try_to_import(self, modname: str) -> str:
        return self.try_to_import_many([modname])

    def try_to_import_many(self, modnames: list[str]) -> str:
        testfile_path = self.path / "test.py"
        testfile_path.write_text(
            "\n\n".join(
                # NB: Remember we don't _actually_ install anything, so
                # every import will be an `ImportError`
                f"try:import {modname}\nexcept ImportError:pass"
                for modname in modnames
            )
        )

        subprocess.check_call(
            [str(self.python_path), testfile_path],
            cwd=str(self.path),
        )
        return self._would_installed_path.read_text().splitlines()

@pytest.fixture
def project_dir(tmp_path) -> ProjectDir:
    subprocess.check_call(
        ["uv", "venv", ".venv", "-q"],
        cwd=tmp_path,
    )
    return ProjectDir(tmp_path, tmp_path / ".venv")


def lockfile(name):
    return Path(__file__, "..", "lockfiles", name).resolve().read_text()

@pytest.mark.parametrize(
    ["modname", "lockfile", "expected_args"],
    [
        pytest.param("direct_dep", "direct_dep", ["direct_dep"], id="simple"),
        pytest.param("direct_dep", "direct_dep == 0.0", ["direct_dep == 0.0"], id="simple_pin"),
        pytest.param("direct_dep", "direct_dep==0.0", ["direct_dep==0.0"], id="simple_pin"),
        pytest.param("direct_dep", "direct_dep @ http://website.com/package", ["direct_dep @ http://website.com/package"], id="url_pin"),
        pytest.param("direct_dep", 'direct_dep @ http://website.com/package ;python_version<"2.7"', ['direct_dep @ http://website.com/package ;python_version<"2.7"'], id="url_pin"),
        pytest.param("direct_dep", "direct_dep[extra1]", ["direct_dep[extra1]"], id="extras"),
        pytest.param("direct_dep", "direct_dep[extra1, extra2]", ["direct_dep[extra1, extra2]"], id="extras"),
        pytest.param("direct_dep", 'direct_dep [security,tests] >= 2.8.1, == 2.8.* ; python_version < "2.7"', ['direct_dep [security,tests] >= 2.8.1, == 2.8.* ; python_version < "2.7"'], id="complicated"),
        pytest.param("direct_dep", "direct_dep\nother_dep", ["direct_dep"], id="superfluous"),
        pytest.param("direct_dep", "other_dep\ndirect_dep", ["direct_dep"], id="superfluous"),
        pytest.param("django", lockfile("pip-compile1.txt"), ["django==4.1.7", "sqlparse==0.4.3", "asgiref==3.6.0"], id="requirements"),
        pytest.param("asgiref", lockfile("pip-compile1.txt"), ["asgiref==3.6.0"], id="requirements"),
        pytest.param("sqlparse", lockfile("pip-compile1.txt"), ["sqlparse==0.4.3"], id="requirements"),
        pytest.param("django", lockfile("pip-compile2.txt"), ["django==4.1.7", "sqlparse==0.4.3", "asgiref==3.6.0"], id="requirements"),
        pytest.param("pydantic", lockfile("pip-compile3.txt"), ["pydantic==2.6.1", "typing_extensions==4.9.0", "pydantic_core==2.16.2", "annotated_types==0.6.0"], id="requirements"),
        pytest.param("typing_extensions", lockfile("pip-compile3.txt"), ["typing_extensions==4.9.0"], id="requirements"),
        pytest.param("rich", lockfile("f2246ce.txt"), ["rich"], id="requirements"),
        pytest.param("keras", lockfile("f2246ce.txt"), ["keras ; python_version < '3.12'"], id="requirements"),
    ]
)
def test_dependency_resolution(project_dir: ProjectDir, modname: str, lockfile: str, expected_args: list[str]):
    """Test that we try and install the correct dependency tree."""
    project_dir.setup(lockfile)
    assert project_dir.try_to_import(modname) == expected_args

@pytest.mark.parametrize(
    ["modname", "module_map", "lockfile", "expected_args"],
    [
        pytest.param("a", {"a": "a"}, "a", ["a"], id="weirdly-identical"),
        pytest.param("modname", {"modname": "package-a"}, "package-a", ["package_a"], id="simple"),
    ]
)
def test_module_map(project_dir: ProjectDir, modname: str, module_map: dict[str, str], lockfile: str, expected_args: list[str]):
    """Test module_map works as expected."""
    project_dir.setup(lockfile, module_map=module_map)
    assert project_dir.try_to_import(modname) == expected_args

@pytest.mark.parametrize(
    ["modname", "package_companions", "lockfile", "expected_args"],
    [
        pytest.param("a", {"a": ["a"]}, "a", ["a"], id="weirdly-identical"),
        pytest.param("package_a", {"package_a": ["package-a", "package-b"]}, "package-a\npackage-b", ["package_a", "package_b"], id="simple"),
    ]
)
def test_package_companions(project_dir: ProjectDir, modname: str, package_companions: dict[str, str], lockfile: str, expected_args: list[str]):
    """Test package_companions works as expected."""
    project_dir.setup(lockfile, package_companions=package_companions)
    assert project_dir.try_to_import(modname) == expected_args


@pytest.mark.parametrize(
    ["modnames", "lockfile", "expected_args"],
    [
        pytest.param(["a"], "a", ["a"], id="simple"),
        pytest.param(["a", "b"], "a\nb", ["a", "b"], id="simple"),
        pytest.param(["a", "a"], "a\nb", ["a"], id="simple"),
        pytest.param(["a", "b"], "a\nb # via a", ["a", "b"], id="simple"),
        pytest.param(["a", "b", "b", "a"], "a\nb # via a", ["a", "b"], id="simple"),  # (You are a DANCING QUEEEEN)
        pytest.param(["b", "a"], "a\nb # via a", ["b", "a"], id="simple"),
    ]
)
def test_multiple_imports(project_dir: ProjectDir, modnames: list[str], lockfile: str, expected_args: list[str]):
    """Test what we do for multiple imports (e.g. don't try and re-install)."""
    project_dir.setup(lockfile)
    assert project_dir.try_to_import_many(modnames) == expected_args

@pytest.mark.parametrize(
        ["modname"],
        [
            ("abc",),
            ("json",),
            ("winreg",),
        ]
)
def test_stdlib(capsys: pytest.CaptureFixture, project_dir: ProjectDir, modname: str):
    project_dir.setup("")
    assert project_dir.try_to_import(modname) == []
    assert not capsys.readouterr().err


@pytest.mark.parametrize(
    ["modname", "module_map", "package_companions", "lockfile", "expected_args"],
    [
        pytest.param("x", {"x": "b"}, {"b": ["c"]}, "a # via b\nb\nc", ["b", "c", "a"], id="simplish"),
    ]
)
def test_everything(project_dir: ProjectDir, modname: str, module_map: dict[str, str], package_companions: dict[str, str], lockfile: str, expected_args: list[str]):
    project_dir.setup(lockfile, package_companions=package_companions, module_map=module_map)
    assert project_dir.try_to_import(modname) == expected_args

@pytest.mark.parametrize(
    ["modname", "lockfile", "expected_args"],
    [
        pytest.param("requests", "REQUESTS==3.0", ["requests==3.0"], id="simplish"),
    ]
)
def test_incorrect_version_installed(project_dir: ProjectDir, modname: str, lockfile: str, expected_args: list[str]):
    project_dir.setup(lockfile, enforce_package_versions=True)
    subprocess.check_call(
        ["uv", "pip", "install", "--no-deps", "requests==2.31.0"],
        env={
            **os.environ.copy(),
            "VIRTUAL_ENV": str(project_dir.venv_dir.resolve()),
        }
    )
    assert project_dir.try_to_import(modname) == expected_args

def test_handles_dotted_import():
    ...
