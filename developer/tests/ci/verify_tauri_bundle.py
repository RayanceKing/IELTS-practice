#!/usr/bin/env python3
"""Fail a release build when Tauri produced no verifiable bundle artifacts."""
from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_REPORT = ROOT / "developer/tests/e2e/reports/tauri-bundle-manifest.json"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target-root", default=str(ROOT / "target"))
    parser.add_argument("--report", default=str(DEFAULT_REPORT))
    args = parser.parse_args()

    target_root = Path(args.target_root).resolve()
    report_path = Path(args.report).resolve()
    bundle_dirs = sorted(path for path in target_root.glob("**/release/bundle") if path.is_dir())
    files = sorted(
        path
        for bundle_dir in bundle_dirs
        for path in bundle_dir.rglob("*")
        if path.is_file()
    )

    artifacts = []
    for path in files:
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        artifacts.append(
            {
                "path": path.relative_to(ROOT).as_posix(),
                "size": path.stat().st_size,
                "sha256": digest.hexdigest(),
            }
        )

    report = {
        "schemaVersion": 1,
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "status": "passed" if artifacts else "failed",
        "targetRoot": str(target_root),
        "artifactCount": len(artifacts),
        "artifacts": artifacts,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if artifacts else 1


if __name__ == "__main__":
    raise SystemExit(main())
