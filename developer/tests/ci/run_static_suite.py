#!/usr/bin/env python3
"""Phase 10 static gate for the shipping Tauri 2 application.

This gate deliberately ignores the retired root HTML/Electron/Fastify host.
It verifies the only shipping path: Vue build -> Tauri frontendDist -> Rust
workspace, plus source reading-data integrity while that migration is active.
"""
from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
REPORT = ROOT / "developer/tests/e2e/reports/static-ci-report.json"

for stream in (sys.stdout, sys.stderr):
    try:
        stream.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError):
        pass


def run_command(name: str, command: list[str], cwd: Path = ROOT) -> dict[str, Any]:
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=300,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"name": name, "status": "fail", "detail": str(exc)}

    output = "\n".join(part.strip() for part in (completed.stdout, completed.stderr) if part.strip())
    return {
        "name": name,
        "status": "pass" if completed.returncode == 0 else "fail",
        "exitCode": completed.returncode,
        "detail": output[-4000:],
    }


def check_tauri_contract() -> dict[str, Any]:
    config_path = ROOT / "src-tauri/tauri.conf.json"
    try:
        config = json.loads(config_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return {"name": "Tauri shipping contract", "status": "fail", "detail": str(exc)}

    build = config.get("build") or {}
    failures: list[str] = []
    if build.get("frontendDist") != "../dist/writing":
        failures.append("frontendDist must be ../dist/writing")
    if not str(build.get("beforeBuildCommand") or "").startswith("npm --prefix apps/writing-vue"):
        failures.append("beforeBuildCommand must build apps/writing-vue")
    serialized = json.dumps(config).lower()
    for retired in ("electron", "fastify", "file://"):
        if retired in serialized:
            failures.append(f"retired host reference in tauri.conf.json: {retired}")

    return {
        "name": "Tauri shipping contract",
        "status": "fail" if failures else "pass",
        "detail": failures or "Vue frontendDist and Tauri-only host contract verified",
    }


def check_required_sources() -> dict[str, Any]:
    required = [
        ROOT / "apps/writing-vue/package.json",
        ROOT / "apps/writing-vue/src/main.js",
        ROOT / "src-tauri/Cargo.toml",
        ROOT / "src-tauri/src/main.rs",
        ROOT / "crates/ielts-domain/Cargo.toml",
        ROOT / "crates/ielts-db/Cargo.toml",
    ]
    missing = [str(path.relative_to(ROOT)) for path in required if not path.is_file()]
    return {
        "name": "Shipping source layout",
        "status": "fail" if missing else "pass",
        "detail": {"missing": missing},
    }


def main() -> int:
    checks = [
        check_required_sources(),
        check_tauri_contract(),
        run_command("Vue typecheck", ["npm.cmd", "--prefix", "apps/writing-vue", "run", "typecheck"]),
        run_command(
            "Reading payload contract",
            ["node", "developer/tests/js/readingAssetPayloadShape.test.mjs"],
        ),
        run_command(
            "Reading drag keyboard behavior",
            ["node", "developer/tests/js/readingDragSelection.test.mjs"],
        ),
        run_command(
            "Reading highlight core",
            ["node", "developer/tests/js/readingHighlightCore.test.mjs"],
        ),
        run_command(
            "Reading mode flow core",
            ["node", "developer/tests/js/readingModeFlowCore.test.mjs"],
        ),
        run_command("Vue production build", ["npm.cmd", "--prefix", "apps/writing-vue", "run", "build"]),
        run_command("Rust workspace check", ["cargo", "check", "--workspace", "--locked"]),
        run_command(
            "AI configuration security",
            [sys.executable, "developer/tests/ci/check_ai_config_security.py"],
        ),
        run_command(
            "Reading source data integrity",
            [sys.executable, "developer/tests/ci/check_reading_data_integrity.py"],
        ),
    ]
    passed = all(check["status"] == "pass" for check in checks)
    report = {
        "schemaVersion": 2,
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "scope": "tauri-vue-shipping-baseline",
        "status": "pass" if passed else "fail",
        "summary": {
            "total": len(checks),
            "passed": sum(check["status"] == "pass" for check in checks),
            "failed": sum(check["status"] == "fail" for check in checks),
        },
        "checks": checks,
        "excludedRetiredHosts": ["Electron", "Fastify", "root index.html", "file:// E2E host"],
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
