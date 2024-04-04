from dataclasses import dataclass
from pathlib import Path
import subprocess
import sys
import textwrap

import pytest


@dataclass(frozen=True)
class ProjectDir:
    path: Path
    venv_dir: Path

    @property
    def python_path(self) -> Path:
        return self.venv_dir / "bin" / "python"

    def install_impending(self) -> None:
        # @TODO: Explain hacks
        site_sir = subprocess.check_output(
            [str(self.python_path), "-c", 'print(__import__("site").getsitepackages()[0])'],
            text=True,
        ).strip()
        Path(site_sir, "impending.pth").write_text(str(Path(__file__).parent.parent / "python"))
        Path(site_sir, "_impending.pth").write_text((Path(__file__).parent.parent / "python" / "_impending.pth").read_text())

    def write_tree(self, tree: dict[str, str]) -> None:
        for path, contents in tree.items():
            Path(self.path, path).parent.mkdir(exist_ok=True, parents=True)
            Path(self.path, path).write_text(textwrap.dedent(contents))
