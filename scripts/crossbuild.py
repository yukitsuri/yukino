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
            m = re.search(r'version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    print("[-] Error: Could not extract version from Cargo.toml")
    sys.exit(1)

def selectTargets(availableTargets: list[str]) -> list[str]:
    print("\n  Select targets:\n")
    for i, t in enumerate(availableTargets, 1):
        print(f"  {i}. {t}")
    try:
        reply = input("\n  Enter numbers separated by commas, or 0 for all: ").strip()
    except EOFError:
        reply = "0"
    if reply == "0" or not reply:
        return availableTargets
    filteredTargets = []
    try:
        choices = [int(x.strip()) for x in reply.split(",") if x.strip()]
        for choice in choices:
            if 1 <= choice <= len(availableTargets):
                candidate = availableTargets[choice - 1]
                if candidate not in filteredTargets:
                    filteredTargets.append(candidate)
            else:
                print(f"[-] Invalid target number: {choice}")
    except ValueError:
        print("[-] Invalid input format. Exiting.")
        sys.exit(1)
    if not filteredTargets:
        print("[-] No targets selected. Exiting.")
        sys.exit(0)
    return filteredTargets

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

def findNdkClang() -> Path | None:
    candidates: list[Path] = []
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
            clang = ndkDir / "toolchains" / "llvm" / "prebuilt" / "linux-x86_64" / "bin" / "aarch64-linux-android26-clang"
            if clang.exists():
                return clang
    return None

def buildSingleTarget(target: str, yukinoDir: Path, buildDir: Path, env: dict[str, str]) -> tuple[bool, str | None]:
    print("=" * 50)
    print(f"[*] Building for target: {target}")
    print("=" * 50)
    # ensure rust stdlib component is available
    subprocess.run(["rustup", "target", "add", target], check=False)
    targetEnv = env.copy()
    cargoCmd = ["cargo", "zigbuild"]
    if "windows" in target or target == "x86_64-unknown-linux-gnu":
        cargoCmd = ["cargo", "build"]
    elif "android" in target:
        ndkClang = findNdkClang()
        if ndkClang:
            targetEnv["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"] = str(ndkClang)
            targetEnv["CC_aarch64_linux_android"] = str(ndkClang)
            cargoCmd = ["cargo", "build"]
    ext = ".exe" if "windows" in target else ""
    hashes = []
    for binary in ["yukino", "yukic"]:
        cmd = cargoCmd + ["--release", "--target", target, "--bin", binary]
        print(f"[*] Running: {' '.join(cmd)}")
        res = subprocess.run(cmd, env=targetEnv)
        if res.returncode != 0:
            print(f"[-] Failed to build {binary} for {target}. Skipping...")
            return (False, None)
        binSrc = yukinoDir / "target" / target / "release" / f"{binary}{ext}"
        if not binSrc.exists():
            print(f"[-] Error: Binary not found at {binSrc}. Skipping...")
            return (False, None)
        binDir = buildDir / binary
        binDir.mkdir(parents=True, exist_ok=True)
        binDest = binDir / f"{binary}-{target}{ext}"
        print(f"[*] Copying binary to {binDest}")
        shutil.copy2(binSrc, binDest)
    if "android" in target:
        ndkClang = findNdkClang()
        if ndkClang:
            stripTool = ndkClang.parent / "llvm-strip"
            for binary in ["yukino", "yukic"]:
                binDest = buildDir / binary / f"{binary}-{target}{ext}"
                if stripTool.exists():
                    subprocess.run([str(stripTool), str(binDest)], check=False)
    for binary in ["yukino", "yukic"]:
        binDest = buildDir / binary / f"{binary}-{target}{ext}"
        hashes.append(hashlib.sha256(binDest.read_bytes()).hexdigest())
    fileHash = ", ".join(hashes)
    print(f"[+] Successfully built and stripped {target}")
    return (True, fileHash)

def printSummary(successful: list[tuple[str, str]], failed: list[str]) -> None:
    print("\n" + "=" * 50)
    print("                 BUILD SUMMARY")
    print("=" * 50)
    if successful:
        print("[+] Successful builds:")
        for t, h in successful:
            print(f"    - {t} (sha256: {h})")
    else:
        print("[-] No successful builds.")
    if failed:
        print("\n[-] Failed builds:")
        for t in failed:
            print(f"    - {t}")
    print("=" * 50)

def main() -> None:
    scriptDir = Path(__file__).resolve().parent
    yukinoDir = scriptDir.parent
    os.chdir(yukinoDir)
    cargoToml = yukinoDir / "Cargo.toml"
    version = readVersion(cargoToml)
    buildDir = yukinoDir / "build" / version
    buildDir.mkdir(parents=True, exist_ok=True)
    availableTargets = [
        "x86_64-pc-windows-gnu",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "aarch64-linux-android",
    ]
    filteredTargets = selectTargets(availableTargets)
    env = setupEnvironment()
    successfulTargets: list[tuple[str, str]] = []
    failedTargets: list[str] = []
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
