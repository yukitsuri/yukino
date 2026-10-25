#!/usr/bin/env python3
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys


def installDirectory() -> Path:
    if os.name == "nt":
        return Path(os.environ.get("USERPROFILE", Path.home())) / ".cargo" / "bin"
    return Path.home() / ".local" / "bin"


def installBinary(source: Path, destination: Path) -> None:
    if destination.exists():
        print(f"warn: bin with same name aleardy installed, reinstall: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    if os.name == "nt":
        shutil.copy2(source, destination)
        return
    installTool = shutil.which("install")
    if installTool:
        subprocess.run([installTool, "-m", "755", str(source), str(destination)], check=True)
        return
    shutil.copy2(source, destination)
    destination.chmod(destination.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def main() -> None:
    scriptDirectory = Path(__file__).resolve().parent
    projectDirectory = scriptDirectory.parent
    environment = os.environ.copy()
    environment["YUKINO_RELEASEBUILD"] = "0"
    command = ["cargo", "build", "--bin", "yukino", "--bin", "yukic"]
    subprocess.run(command, cwd=projectDirectory, env=environment, check=True)

    extension = ".exe" if os.name == "nt" else ""
    binaryDirectory = projectDirectory / "target" / "debug"
    destinationDirectory = installDirectory()
    for name in ["yukino", "yukic"]:
        source = binaryDirectory / f"{name}{extension}"
        destination = destinationDirectory / f"{name}-dbg{extension}"
        if not source.is_file():
            raise RuntimeError(f"built binary not found: {source}")
        installBinary(source, destination)
        print(f"installed: {destination}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        print(f"error: command failed with status {error.returncode}", file=sys.stderr)
        sys.exit(error.returncode)
    except KeyboardInterrupt:
        print("error: build interrupted", file=sys.stderr)
        sys.exit(130)
