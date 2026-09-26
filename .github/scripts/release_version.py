"""Apply a release version to the CI workspace without changing the repository."""

import os
import re
import subprocess
import sys
from pathlib import Path


VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?\Z")
PACKAGE = re.compile(r"(?ms)^\[package\][ \t]*\n(?P<body>.*?)(?=^\[|\Z)")
PACKAGE_VERSION = re.compile(r'^version[ \t]*=[ \t]*"[^"]+"[ \t]*$', re.MULTILINE)


def main() -> None:
    requested = sys.argv[1]
    manual = os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch"
    version = requested if manual else requested.removeprefix("v")
    if not VERSION.fullmatch(version) or (manual and requested.startswith("v")):
        raise SystemExit("version must look like 0.8.0 or 0.8.0-rc.1, without a v prefix for manual releases")

    tag = requested
    if manual:
        result = subprocess.run(
            ["git", "ls-remote", "--exit-code", "--tags", "origin", f"refs/tags/{tag}"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            raise SystemExit(f"release tag {tag} already exists")
        if result.returncode != 2:
            raise SystemExit(f"could not check release tag {tag}: {result.stderr.strip()}")

    manifest = Path(os.environ.get("OCIPACK_MANIFEST_PATH", "Cargo.toml"))
    original = manifest.read_text(encoding="utf-8")
    package = PACKAGE.search(original)
    if package is None:
        raise SystemExit("Cargo.toml has no [package] section")
    match = PACKAGE_VERSION.search(package.group("body"))
    if match is None:
        raise SystemExit("Cargo.toml has no package version")
    start = package.start("body") + match.start()
    end = package.start("body") + match.end()
    manifest.write_text(original[:start] + f'version = "{version}"' + original[end:], encoding="utf-8")

    env_file = os.environ.get("GITHUB_ENV")
    if env_file:
        with open(env_file, "a", encoding="utf-8") as output:
            output.write(f"RELEASE_VERSION={tag}\n")
    print(f"Building {tag} (temporary Cargo package version {version})")


if __name__ == "__main__":
    main()
