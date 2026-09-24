#!/usr/bin/env python3
"""Reject removed inference dependencies and ratchet remaining cloud source references."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / "script/local_only_residue.json"
REMOVED = {
    "ort", "ort-sys", "candle-core", "candle-nn", "candle-onnx", "tokenizers",
    "serve-wasm", "managed_secrets_wasm",
    "minidumper", "crash-handler",
}
RESIDUE = {
    "ai", "ai_types", "mcp", "input_classifier", "natural_language_detection",
    "cloud_objects", "cloud_object_client", "cloud_object_models",
    "cloud_object_persistence", "firebase", "warp_graphql", "warp_graphql_schema",
    "warp_server_auth", "warp_server_client", "warp_multi_agent_api",
    "warp_multi_agent_client", "remote_server", "computer_use", "voice_input",
    "oauth2", "cynic", "graphql-ws-client",
}
ENDPOINT = re.compile(
    r"(?:https?|wss?)://[^\s\"'<>]*"
    r"(?:\.warp\.dev|\.warp\.work|\.sentry\.io|\.firebaseio\.com|"
    r"identitytoolkit\.googleapis\.com|securetoken\.googleapis\.com)"
    r"|AIza[0-9A-Za-z_-]{35}",
    re.IGNORECASE,
)
SOURCE_SUFFIXES = {".rs", ".toml", ".sh", ".ps1", ".yml", ".yaml", ".json"}


def removed_dependency(name):
    return name in REMOVED or name.startswith(("opentelemetry", "tracing-opentelemetry", "sentry"))


def restricted(name):
    return name in RESIDUE


def dependencies(value):
    for key, item in value.items():
        if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
            for alias, spec in item.items():
                yield spec.get("package", alias) if isinstance(spec, dict) else alias
        elif isinstance(item, dict):
            yield from dependencies(item)


def inventory(root, paths):
    found = Counter()
    forbidden = set()
    for relative in paths:
        path = root / relative
        if not path.is_file():
            continue
        if path.name == "Cargo.toml":
            for name in dependencies(tomllib.loads(path.read_text())):
                if removed_dependency(name):
                    forbidden.add(f"{relative}: {name}")
                elif restricted(name):
                    found[f"dependency:{relative}:{name}"] += 1
        if path.suffix in SOURCE_SUFFIXES and relative.split("/")[0] in {
            "app", "crates", "script", ".github", ".cargo", "resources",
        }:
            content = path.read_text(errors="replace")
            for line in content.splitlines():
                if ENDPOINT.search(line):
                    digest = hashlib.sha256(line.strip().encode()).hexdigest()
                    found[f"endpoint:{relative}:{digest}"] += 1
    for package in tomllib.loads((root / "Cargo.lock").read_text())["package"]:
        name = package["name"]
        if removed_dependency(name):
            forbidden.add(f"Cargo.lock: {name}")
        elif restricted(name):
            found[f"lock:{name}:{package['version']}"] += 1
    return dict(sorted(found.items())), sorted(forbidden)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", action="store_true", help="Print hashed residue for review")
    args = parser.parse_args()
    paths = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=ROOT
    ).decode().split("\0")
    actual, forbidden = inventory(ROOT, sorted(set(filter(None, paths))))
    if forbidden:
        print("Removed dependencies returned:\n" + "\n".join(forbidden), file=sys.stderr)
        return 1
    if args.inventory:
        print(json.dumps(actual, indent=2, sort_keys=True))
        return 0
    expected = json.loads(BASELINE.read_text())
    if actual != expected:
        added = Counter(actual) - Counter(expected)
        removed = Counter(expected) - Counter(actual)
        print(f"Cloud residue changed: {sum(added.values())} added, {sum(removed.values())} removed.")
        for key in added:
            print(f"Forbidden addition: {key}")
        print("Review removals and shrink the residue inventory; never accept new cloud entries.")
        return 1
    print(f"Local-only guard passed; {sum(actual.values())} explicitly tracked residue entries remain.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
