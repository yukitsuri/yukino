#!/usr/bin/env python3
import os
import shutil
import subprocess
import sys
import tempfile

def findCompiler():
    for name in ["yukic", "yukic-dbg"]:
        path = shutil.which(name)
        if path:
            return path
    defaultPath = os.path.expanduser("~/.local/bin/yukic-dbg")
    if os.path.exists(defaultPath) and os.access(defaultPath, os.X_OK):
        return defaultPath
    return None

def main():
    if len(sys.argv) < 2:
        sys.stderr.write("usage: test_snippet.py <source_file.ykn>\n")
        sys.exit(1)

    sourcePath = os.path.abspath(sys.argv[1])
    if not os.path.exists(sourcePath):
        sys.stderr.write("source file not found: " + sourcePath + "\n")
        sys.exit(1)

    compiler = findCompiler()
    if not compiler:
        sys.stderr.write("compiler binary (yukic-dbg or yukic) not found in PATH\n")
        sys.exit(1)

    tempDir = tempfile.mkdtemp(prefix="yukino_snippet_")
    outBinary = os.path.join(tempDir, "out_bin")
    try:
        cmd = [compiler, sourcePath, "-o", outBinary]
        result = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )

        if result.stdout:
            sys.stdout.write(result.stdout)
        if result.stderr:
            sys.stderr.write(result.stderr)

        if result.returncode != 0:
            sys.stderr.write("compilation failed with exit code " + str(result.returncode) + "\n")
            sys.exit(result.returncode)

        print("compilation successful")
        if "--run" in sys.argv and os.path.exists(outBinary):
            print("--- running binary ---")
            runResult = subprocess.run([outBinary])
            sys.exit(runResult.returncode)

    finally:
        shutil.rmtree(tempDir, ignore_errors=True)

if __name__ == "__main__":
    main()
