#!/usr/bin/env python3
"""Static security contract for the Rust-owned AI configuration boundary."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
VUE_ROOT = ROOT / "apps/writing-vue/src"
AI_RUST = ROOT / "src-tauri/src/commands/ai.rs"
TAURI_LIB = ROOT / "src-tauri/src/lib.rs"
CLIENT = ROOT / "apps/writing-vue/src/api/client.js"
SETTINGS_REPOSITORY = ROOT / "apps/writing-vue/src/api/settings-repository.js"
FIXTURE = ROOT / "developer/tests/fixtures/legacy-provider-configs.json"

RUNTIME_KEYS = ("provider", "baseUrl", "model", "secretName", "timeoutSeconds")
AI_COMMANDS = (
    "ai_list_configs",
    "ai_upsert_config",
    "ai_delete_config",
    "ai_set_default_config",
    "ai_test_provider",
)


def fail(message: str, failures: list[str]) -> None:
    failures.append(message)


def shipping_sources() -> list[Path]:
    suffixes = {".js", ".ts", ".vue", ".cjs", ".mjs"}
    return [path for path in VUE_ROOT.rglob("*") if path.is_file() and path.suffix in suffixes]


def check_no_web_storage_secret(failures: list[str]) -> None:
    storage = re.compile(r"(?:localStorage|sessionStorage)", re.IGNORECASE)
    secret = re.compile(r"api[_-]?key|apikey|secret|password|bearer", re.IGNORECASE)
    for path in shipping_sources():
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        for index, line in enumerate(lines):
            if not storage.search(line):
                continue
            window = "\n".join(lines[max(0, index - 3) : index + 4])
            if secret.search(window):
                fail(f"{path.relative_to(ROOT)}:{index + 1}: secret material near Web Storage", failures)


def check_rust_owned_crud(failures: list[str]) -> None:
    client = CLIENT.read_text(encoding="utf-8", errors="replace")
    repository = SETTINGS_REPOSITORY.read_text(encoding="utf-8", errors="replace")
    for command in AI_COMMANDS[:-1]:
        if command not in repository:
            fail(f"frontend provider CRUD does not invoke Rust command {command}", failures)
    provider_block = re.search(r"export const configs\s*=\s*\{(?P<body>.*?)\n\}", client, re.DOTALL)
    if provider_block and re.search(r"(?:readKvList|writeKv|deleteKv)\(['\"]provider_configs", provider_block.group("body")):
        fail("frontend provider CRUD still persists provider_configs through generic SQLite settings", failures)

    if re.search(r"(?:writeKv|upsertSetting)[^\n]*(?:api_key|apiKey)", client + repository):
        fail("frontend persists plaintext API key through generic settings storage", failures)


def check_runtime_contract(failures: list[str]) -> None:
    rust = AI_RUST.read_text(encoding="utf-8", errors="replace")
    registered = TAURI_LIB.read_text(encoding="utf-8", errors="replace")
    for key in RUNTIME_KEYS:
        if f'"{key}"' not in rust:
            fail(f"Rust active AI config is missing runtime key {key}", failures)
    for command in AI_COMMANDS:
        if f"commands::ai::{command}" not in registered:
            fail(f"Tauri invoke handler does not register {command}", failures)


def check_legacy_fixture(failures: list[str]) -> None:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    legacy = fixture.get("legacy") or []
    expected = fixture.get("expectedRuntimeSettings") or {}
    forbidden = set(fixture.get("forbiddenPersistedKeys") or [])
    if not legacy or not any("api_key" in row for row in legacy):
        fail("legacy provider fixture must contain a plaintext api_key regression sample", failures)
    if set(RUNTIME_KEYS) != set(expected):
        fail("legacy provider fixture must map exactly to the complete Rust runtime key set", failures)
    leaked = forbidden.intersection(expected)
    if leaked:
        fail(f"legacy migration expectation persists forbidden keys: {sorted(leaked)}", failures)


def main() -> int:
    failures: list[str] = []
    check_no_web_storage_secret(failures)
    check_rust_owned_crud(failures)
    check_runtime_contract(failures)
    check_legacy_fixture(failures)
    if failures:
        print("AI configuration security gate failed:")
        for item in failures:
            print(f"- {item}")
        return 1
    print("AI configuration security gate passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
