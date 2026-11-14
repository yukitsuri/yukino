#!/usr/bin/env python3
import argparse
import os
import re
import sys

def searchFile(filePath, queryRegex, contextLines):
    matches = []
    try:
        with open(filePath, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except (IOError, UnicodeDecodeError):
        return matches

    currentHeader = ""
    for i, line in enumerate(lines):
        headerMatch = re.match(r"^(#{1,6})\s+(.*)$", line)
        if headerMatch:
            currentHeader = headerMatch.group(2).strip()

        if queryRegex.search(line):
            startIdx = max(0, i - contextLines)
            endIdx = min(len(lines), i + contextLines + 1)
            snippet = "".join(lines[startIdx:endIdx]).rstrip()
            matches.append({
                "line": i + 1,
                "header": currentHeader,
                "snippet": snippet
            })
    return matches

def runSearch(baseDir, query, isCaseSensitive, isStdlibOnly, contextLines):
    flags = 0 if isCaseSensitive else re.IGNORECASE
    try:
        queryRegex = re.compile(query, flags)
    except re.error as err:
        sys.stderr.write("invalid regex query: " + str(err) + "\n")
        sys.exit(1)

    targetDir = os.path.join(baseDir, "stdlib") if isStdlibOnly else baseDir
    results = {}

    for root, _, files in os.walk(targetDir):
        for fileName in sorted(files):
            if not fileName.endswith(".md"):
                continue
            fullPath = os.path.join(root, fileName)
            relPath = os.path.relpath(fullPath, baseDir)
            matchedItems = searchFile(fullPath, queryRegex, contextLines)
            if matchedItems:
                results[relPath] = matchedItems

    return results

def main():
    parser = argparse.ArgumentParser(
        description="Search through Yukino language and standard library references."
    )
    parser.add_argument("query", help="Regex or literal string to search for")
    parser.add_argument(
        "--stdlib-only", "-s", action="store_true", help="Search only stdlib docs"
    )
    parser.add_argument(
        "--case-sensitive", "-c", action="store_true", help="Enable case-sensitive matching"
    )
    parser.add_argument(
        "--context", "-C", type=int, default=1, help="Context lines around match (default 1)"
    )

    args = parser.parse_args()
    scriptDir = os.path.dirname(os.path.abspath(__file__))
    referencesDir = os.path.abspath(os.path.join(scriptDir, "..", "references", "en"))

    if not os.path.exists(referencesDir):
        sys.stderr.write("references directory not found: " + referencesDir + "\n")
        sys.exit(1)

    results = runSearch(
        referencesDir,
        args.query,
        args.case_sensitive,
        args.stdlib_only,
        args.context
    )

    totalMatches = sum(len(items) for items in results.values())
    if totalMatches == 0:
        print("no matches found for: " + args.query)
        return

    print("found " + str(totalMatches) + " matches across " + str(len(results)) + " files:\n")
    for relPath, items in results.items():
        print("=== " + relPath + " (" + str(len(items)) + " hits) ===")
        for item in items:
            headerInfo = " [" + item["header"] + "]" if item["header"] else ""
            print("  line " + str(item["line"]) + headerInfo + ":")
            for snippetLine in item["snippet"].splitlines():
                print("    " + snippetLine)
            print()

if __name__ == "__main__":
    main()
