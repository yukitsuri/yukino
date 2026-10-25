#!/usr/bin/env python3
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


def readVersion(cargoToml: Path) -> str:
    if not cargoToml.exists():
        print("[-] Error: Cargo.toml not found")
        sys.exit(1)
    with open(cargoToml, "r", encoding="utf-8") as f:
        for line in f:
            match = re.search(r'version\s*=\s*"([^"]+)"', line)
            if match:
                return match.group(1)
    print("[-] Error: Could not extract version from Cargo.toml")
    sys.exit(1)


def selectTargets(availableTargets: list[str]) -> list[str]:
    print("\n  Select targets:\n")
    for i, target in enumerate(availableTargets, 1):
        print(f"  {i}. {target}")
    try:
        reply = input("\n  Enter numbers separated by commas, or 0 for all: ").strip()
    except EOFError:
        reply = "0"
    if reply == "0" or not reply:
        return availableTargets
    selectedTargets = []
    try:
        choices = [int(item.strip()) for item in reply.split(",") if item.strip()]
        for choice in choices:
            if 1 <= choice <= len(availableTargets):
                target = availableTargets[choice - 1]
                if target not in selectedTargets:
                    selectedTargets.append(target)
            else:
                print(f"[-] Invalid target number: {choice}")
    except ValueError:
        print("[-] Invalid input format. Exiting.")
        sys.exit(1)
    if not selectedTargets:
        print("[-] No targets selected. Exiting.")
        sys.exit(0)
    return selectedTargets


def setupEnvironment() -> dict[str, str]:
    env = os.environ.copy()
    env["YUKINO_RELEASEBUILD"] = "1"
    env["CARGO_PROFILE_RELEASE_LTO"] = "fat"
    env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "1"
    env["CARGO_PROFILE_RELEASE_PANIC"] = "abort"
    env["CARGO_PROFILE_RELEASE_STRIP"] = "true"
    env["RUSTFLAGS"] = "-C symbol-mangling-version=v0"
    cargoBin = Path.home() / ".cargo" / "bin"
    if cargoBin.exists() and str(cargoBin) not in env.get("PATH", ""):
        env["PATH"] = f"{cargoBin}:{env.get('PATH', '')}"
    return env


def findNdkClang(target: str) -> Path | None:
    clangNames = {
        "aarch64-linux-android": "aarch64-linux-android26-clang",
        "armv7-linux-androideabi": "armv7a-linux-androideabi26-clang",
    }
    clangName = clangNames[target]
    candidates = []
    if "ANDROID_NDK_HOME" in os.environ:
        candidates.append(Path(os.environ["ANDROID_NDK_HOME"]))
    if "ANDROID_HOME" in os.environ:
        candidates.append(Path(os.environ["ANDROID_HOME"]) / "ndk")
    candidates.append(Path.home() / "Android" / "Sdk" / "ndk")
    for base in candidates:
        if not base.is_dir():
            continue
        ndkDirs = [base] if (base / "toolchains").exists() else sorted(base.iterdir(), reverse=True)
        for ndkDir in ndkDirs:
            clang = ndkDir / "toolchains" / "llvm" / "prebuilt" / "linux-x86_64" / "bin" / clangName
            if clang.exists():
                return clang
    return None


def buildSingleTarget(target: str, yukinoDir: Path, buildDir: Path, env: dict[str, str]) -> tuple[bool, str | None]:
    print("=" * 50)
    print(f"[*] Building runtime for target: {target}")
    print("=" * 50)
    subprocess.run(["rustup", "target", "add", target], check=False)
    targetEnv = env.copy()
    cargoCmd = ["cargo", "zigbuild"]
    if "windows" in target or target == "x86_64-unknown-linux-gnu":
        cargoCmd = ["cargo", "build"]
    elif "android" in target:
        ndkClang = findNdkClang(target)
        targetKey = {
            "aarch64-linux-android": "AARCH64_LINUX_ANDROID",
            "armv7-linux-androideabi": "ARMV7_LINUX_ANDROIDEABI",
        }[target]
        ccKey = targetKey.lower()
        if ndkClang:
            targetEnv[f"CARGO_TARGET_{targetKey}_LINKER"] = str(ndkClang)
            targetEnv[f"CC_{ccKey}"] = str(ndkClang)
            cargoCmd = ["cargo", "build"]
    cmd = cargoCmd + ["--release", "--target", target, "--package", "yukino-rt"]
    print(f"[*] Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, env=targetEnv)
    if result.returncode != 0:
        print(f"[-] Failed to build runtime for {target}. Skipping...")
        return (False, None)
    archiveSrc = yukinoDir / "target" / target / "release" / "libyukino_rt.a"
    if not archiveSrc.exists():
        print(f"[-] Error: Runtime archive not found at {archiveSrc}. Skipping...")
        return (False, None)
    archiveDest = buildDir / f"{target}.a"
    print(f"[*] Copying runtime archive to {archiveDest}")
    shutil.copy2(archiveSrc, archiveDest)
    fileHash = hashlib.sha256(archiveDest.read_bytes()).hexdigest()
    print(f"[+] Successfully built runtime for {target}")
    return (True, fileHash)


def printSummary(successful: list[tuple[str, str]], failed: list[str]) -> None:
    print("\n" + "=" * 50)
    print("              RUNTIME BUILD SUMMARY")
    print("=" * 50)
    if successful:
        print("[+] Successful builds:")
        for target, fileHash in successful:
            print(f"    - {target} (sha256: {fileHash})")
    else:
        print("[-] No successful builds.")
    if failed:
        print("\n[-] Failed builds:")
        for target in failed:
            print(f"    - {target}")
    print("=" * 50)


def main() -> None:
    scriptDir = Path(__file__).resolve().parent
    yukinoDir = scriptDir.parent
    os.chdir(yukinoDir)
    version = readVersion(yukinoDir / "Cargo.toml")
    buildDir = yukinoDir / "build" / version / "runtime"
    buildDir.mkdir(parents=True, exist_ok=True)
    availableTargets = [
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "armv7-unknown-linux-gnueabihf",
        "armv7-unknown-linux-musleabihf",
        "x86_64-pc-windows-gnu",
        "aarch64-linux-android",
        "armv7-linux-androideabi",
    ]
    filteredTargets = selectTargets(availableTargets)
    env = setupEnvironment()
    successfulTargets = []
    failedTargets = []
    for target in filteredTargets:
        ok, fileHash = buildSingleTarget(target, yukinoDir, buildDir, env)
        if ok and fileHash:
            successfulTargets.append((target, fileHash))
        else:
            failedTargets.append(target)
    printSummary(successfulTargets, failedTargets)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n[-] Build interrupted by user.")
        sys.exit(1)
