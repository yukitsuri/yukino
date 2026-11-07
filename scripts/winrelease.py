#!/usr/bin/env python3
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

# DLLs provided by Windows itself and never bundled.
SYSTEM_DLL_PREFIXES = ("api-ms-", "ext-ms-")
SYSTEM_DLLS = {
    "kernel32.dll",
    "user32.dll",
    "advapi32.dll",
    "ws2_32.dll",
    "shell32.dll",
    "ole32.dll",
    "oleaut32.dll",
    "uuid.dll",
    "msvcrt.dll",
    "ntdll.dll",
    "userenv.dll",
    "dbghelp.dll",
    "psapi.dll",
    "bcrypt.dll",
    "bcryptprimitives.dll",
}


def readVersion(cargoToml: Path) -> str:
    if not cargoToml.exists():
        print("[-] Error: Cargo.toml not found")
        sys.exit(1)
    with open(cargoToml, "r", encoding="utf-8") as f:
        for line in f:
            m = re.search(r'version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    print("[-] Error: Could not extract version from Cargo.toml")
    sys.exit(1)


def isSystemDll(name: str) -> bool:
    lowered = name.lower()
    return lowered in SYSTEM_DLLS or lowered.startswith(SYSTEM_DLL_PREFIXES)


def listImports(exePath: Path) -> list[str]:
    readobj = shutil.which("llvm-readobj")
    if readobj:
        res = subprocess.run(
            [readobj, "--coff-imports", str(exePath)],
            capture_output=True,
            text=True,
            check=False,
        )
        if res.returncode == 0:
            names = re.findall(r"Name:\s+(\S+\.dll)", res.stdout)
            return sorted({n.lower() for n in names if not isSystemDll(n)})
    objdump = shutil.which("objdump")
    if objdump:
        res = subprocess.run(
            [objdump, "-p", str(exePath)],
            capture_output=True,
            text=True,
            check=False,
        )
        if res.returncode == 0:
            names = re.findall(r"DLL Name:\s+(\S+\.dll)", res.stdout)
            return sorted({n.lower() for n in names if not isSystemDll(n)})
    print("[-] Error: neither llvm-readobj nor objdump is available in PATH")
    sys.exit(1)


def findDll(name: str, searchDirs: list[Path]) -> Path:
    for directory in searchDirs:
        candidate = directory / name
        if candidate.is_file():
            return candidate
    print(f"[-] Error: required DLL not found: {name} (searched LLVM_SYS_221_PREFIX/bin and PATH)")
    sys.exit(1)


def collectRuntimeDlls(binaries: list[Path], searchDirs: list[Path]) -> list[Path]:
    collected: dict[str, Path] = {}
    queue = list(binaries)
    scanned: set[Path] = set()
    while queue:
        current = queue.pop()
        if current in scanned:
            continue
        scanned.add(current)
        for name in listImports(current):
            if name not in collected:
                source = findDll(name, searchDirs)
                collected[name] = source
                queue.append(source)
    return sorted(collected.values(), key=lambda p: p.name)


def main() -> None:
    scriptDir = Path(__file__).resolve().parent
    yukinoDir = scriptDir.parent
    os.chdir(yukinoDir)

    version = readVersion(yukinoDir / "Cargo.toml")
    distDir = yukinoDir / "build" / version / "dist"
    if distDir.exists():
        shutil.rmtree(distDir)
    distDir.mkdir(parents=True, exist_ok=True)

    searchDirs: list[Path] = []
    prefix = os.environ.get("LLVM_SYS_221_PREFIX")
    if prefix:
        searchDirs.append(Path(prefix) / "bin")
    searchDirs += [Path(p) for p in os.environ.get("PATH", "").split(os.pathsep) if p]

    env = os.environ.copy()
    env["YUKINO_RELEASEBUILD"] = "1"
    binaries = []
    for name in ["yukino", "yukic"]:
        cmd = ["cargo", "build", "--release", "--bin", name]
        print(f"[*] Running: {' '.join(cmd)}")
        res = subprocess.run(cmd, env=env)
        if res.returncode != 0:
            print(f"[-] Failed to build {name}")
            sys.exit(1)
        source = yukinoDir / "target" / "release" / f"{name}.exe"
        if not source.is_file():
            print(f"[-] Error: binary not found at {source}")
            sys.exit(1)
        shutil.copy2(source, distDir / f"{name}.exe")
        binaries.append(distDir / f"{name}.exe")

    for dll in collectRuntimeDlls(binaries, searchDirs):
        shutil.copy2(dll, distDir / dll.name)

    installScript = scriptDir / "wininstall.ps1"
    if installScript.is_file():
        shutil.copy2(installScript, distDir / "wininstall.ps1")

    zipPath = yukinoDir / "build" / version / f"yukino-{version}-x86_64-windows.zip"
    with zipfile.ZipFile(zipPath, "w", zipfile.ZIP_DEFLATED) as archive:
        for entry in sorted(distDir.iterdir()):
            archive.write(entry, entry.name)

    contents = ", ".join(sorted(p.name for p in distDir.iterdir()))
    print(f"[+] Release bundle: {zipPath}")
    print(f"[*] Contents: {contents}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n[-] Build interrupted by user.")
        sys.exit(1)
