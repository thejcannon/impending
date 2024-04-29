from dataclasses import dataclass
from pathlib import Path
import subprocess
import textwrap


@dataclass(frozen=True)
class ProjectDir:
    path: Path
    venv_dir: Path

    @property
    def python_path(self) -> Path:
        return self.venv_dir / "bin" / "python"

    def install_impending(self) -> None:
        # @TODO: Explain hacks
        site_dir = subprocess.check_output(
            [
                str(self.python_path),
                "-c",
                'print(__import__("site").getsitepackages()[0])',
            ],
            text=True,
        ).strip()
        python_src_dir = Path(__file__).parent.parent / "python"
        pth_contents = (python_src_dir / "_impending.pth").read_text()
        Path(site_dir, "_impending.pth").write_text(
            f"import sys;sys.path.append('{python_src_dir}');{pth_contents}"
        )

    def write_tree(self, tree: dict[str, str]) -> None:
        for path, contents in tree.items():
            Path(self.path, path).parent.mkdir(exist_ok=True, parents=True)
            Path(self.path, path).write_text(textwrap.dedent(contents))
