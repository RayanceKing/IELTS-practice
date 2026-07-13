#!/usr/bin/env python3
"""Truthful packaged Tauri 2 smoke gate.

This deliberately speaks the WebDriver HTTP protocol directly, so the gate
has no hidden Playwright/file:// fallback.  Set TAURI_APP_BINARY to a built
Tauri executable and ensure tauri-driver plus the platform native driver are
on PATH (or set TAURI_DRIVER/TAURI_NATIVE_DRIVER).
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REPORT = ROOT / "developer/tests/e2e/reports/suite-practice-flow-report.json"


def resolve_executable(env_name: str, names: tuple[str, ...], extra: tuple[Path, ...] = ()) -> str | None:
    """Resolve explicit path, PATH entry, then common Windows install locations."""
    explicit = os.environ.get(env_name)
    if explicit:
        candidate = Path(explicit).expanduser()
        if candidate.is_file():
            return str(candidate)
        found = shutil.which(explicit)
        if found:
            return found
    for name in names:
        found = shutil.which(name)
        if found:
            return found
    for candidate in extra:
        if candidate.is_file():
            return str(candidate)
    return None


def executable_version(path: str | None) -> str | None:
    if not path:
        return None
    try:
        result = subprocess.run([path, "--version"], capture_output=True, text=True, timeout=5, check=False)
        value = (result.stdout or result.stderr).strip()
        return value or None
    except (OSError, subprocess.SubprocessError):
        return None


def blocked(reason: str, missing: list[str]) -> int:
    report = {"schemaVersion": 1, "generatedAt": datetime.now(timezone.utc).isoformat(),
              "status": "blocked", "exitCode": 2, "target": "packaged-tauri-2",
              "reason": reason, "missingDependencies": missing,
              "checks": {"launch": "blocked", "vueRoutes": "blocked", "readingIpc": "blocked",
                         "bundledResources": "blocked", "sqliteRestart": "blocked"}}
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 2


class Driver:
    def __init__(self, base: str): self.base, self.sid = base.rstrip("/"), None
    def call(self, method: str, path: str, body=None):
        req = urllib.request.Request(self.base + path, method=method,
                                      data=None if body is None else json.dumps(body).encode(),
                                      headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=30) as response:
                return json.loads(response.read() or b"{}")
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"WebDriver {method} {path} returned HTTP {exc.code}: {detail}") from exc
    def create(self, app: str):
        value = self.call("POST", "/session", {"capabilities": {"alwaysMatch": {
            "browserName": "wry", "tauri:options": {"application": app}}}})
        self.sid = value.get("sessionId") or value.get("value", {}).get("sessionId")
        if not self.sid: raise RuntimeError(f"WebDriver session failed: {value}")
    def script(self, source, args=None):
        value = self.call("POST", f"/session/{self.sid}/execute/sync", {"script": source, "args": args or []})
        return value.get("value", value)
    def url(self, url): self.call("POST", f"/session/{self.sid}/url", {"url": url})
    def close(self):
        if self.sid:
            try: self.call("DELETE", f"/session/{self.sid}")
            except Exception: pass


def wait_for_vue(driver: Driver, timeout_seconds: int = 30):
    deadline = time.time() + timeout_seconds
    last = None
    while time.time() < deadline:
        last = driver.script("""
            const root = document.querySelector('#app');
            return {
              readyState: document.readyState,
              url: location.href,
              mounted: !!root && root.childElementCount > 0,
              html: document.documentElement.outerHTML.slice(0, 1000)
            };
        """)
        if isinstance(last, dict) and last.get("mounted"):
            return last
        time.sleep(0.25)
    raise RuntimeError(f"Vue root did not mount: {last}")


def main() -> int:
    explicit_app = os.environ.get("TAURI_APP_BINARY")
    app_candidates = (ROOT / "target/release/ielts-practice-tauri.exe",
                      ROOT / "target/debug/ielts-practice-tauri.exe")
    app = Path(explicit_app) if explicit_app else next((path for path in app_candidates if path.is_file()), app_candidates[0])
    tauri = resolve_executable("TAURI_DRIVER", ("tauri-driver",),
                               (Path.home() / ".cargo/bin/tauri-driver.exe",))
    native = resolve_executable(
        "TAURI_NATIVE_DRIVER", ("msedgedriver", "chromedriver"),
        (Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData/Local")) / "IELTSAtlas/webdriver/msedgedriver.exe",
         Path(os.environ.get("PROGRAMFILES(X86)", "C:/Program Files (x86)")) / "Microsoft/Edge/Application/msedgedriver.exe",
         Path(os.environ.get("PROGRAMFILES", "C:/Program Files")) / "Microsoft/Edge/Application/msedgedriver.exe"),
    )
    missing = []
    if not app.is_file(): missing.append(f"packaged executable: {app} (set TAURI_APP_BINARY)")
    if not tauri: missing.append("tauri-driver (install cargo-tauri-driver or set TAURI_DRIVER)")
    if not native: missing.append("msedgedriver/chromedriver (set TAURI_NATIVE_DRIVER)")
    if missing:
        missing.extend(filter(None, [f"tauri-driver version: {executable_version(tauri) or 'unknown'}",
                                     f"native driver version: {executable_version(native) or 'unknown'}"]))
        return blocked("packaged WebView dependencies are unavailable; no fallback is permitted", missing)

    proc = subprocess.Popen([tauri, "--native-driver", native], cwd=ROOT,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    driver = Driver(os.environ.get("TAURI_WEBDRIVER_URL", "http://127.0.0.1:4444"))
    checks = {}
    try:
        for _ in range(30):
            try: driver.call("GET", "/status"); break
            except Exception: time.sleep(1)
        driver.create(str(app.resolve()))
        checks["launch"] = "passed"
        wait_for_vue(driver)
        for route in ("#/writing", "#/settings", "#/history", "#/"):
            reached = driver.script("location.hash=arguments[0]; return location.hash === arguments[0]", [route])
            if not reached: raise RuntimeError(f"Vue hash route failed: {route}")
        checks["vueRoutes"] = "passed"
        result = driver.script("return window.__TAURI_INTERNALS__ ? window.__TAURI_INTERNALS__.invoke('reading_list_assets') : null")
        assets = (result or {}).get("data") if isinstance(result, dict) else None
        if not assets: raise RuntimeError("Tauri IPC bridge unavailable or reading_list_assets returned empty")
        checks["readingIpc"] = "passed"
        asset_id = assets[0].get("id") or assets[0].get("assetId") or assets[0].get("asset_id")
        if not asset_id:
            raise RuntimeError(f"reading_list_assets returned an entry without an id: {assets[0]}")
        payload = driver.script("return window.__TAURI_INTERNALS__.invoke('reading_get_asset_payload', {assetId: arguments[0]})", [asset_id])
        if not isinstance(payload, dict) or not payload.get("ok") or not payload.get("data"):
            raise RuntimeError(f"reading_get_asset_payload failed for {asset_id}: {payload}")
        checks["bundledResources"] = "passed"
        marker = f"e2e-{int(time.time())}"
        saved = driver.script("return window.__TAURI_INTERNALS__.invoke('upsert_setting', {cmd:{namespace:'e2e', key:'restartMarker', value:arguments[0]}})", [marker])
        if not isinstance(saved, dict) or not saved.get("ok"): raise RuntimeError(f"upsert_setting failed: {saved}")
        driver.close()
        driver.create(str(app.resolve()))
        wait_for_vue(driver)
        restored = driver.script("return window.__TAURI_INTERNALS__.invoke('list_settings', {namespace:'e2e'})")
        values = (restored or {}).get("data", []) if isinstance(restored, dict) else []
        checks["sqliteRestart"] = "passed" if any(x.get("key") == "restartMarker" and x.get("value") == marker for x in values) else "failed"
        status = "passed" if all(v == "passed" for v in checks.values()) else "failed"
        report = {"schemaVersion": 1, "generatedAt": datetime.now(timezone.utc).isoformat(),
                  "status": status, "exitCode": 0 if status == "passed" else 1,
                  "target": "packaged-tauri-2", "checks": checks}
    except Exception as exc:
        report = {"schemaVersion": 1, "generatedAt": datetime.now(timezone.utc).isoformat(),
                  "status": "failed", "exitCode": 1, "target": "packaged-tauri-2",
                  "checks": checks, "error": str(exc)}
    finally:
        driver.close(); proc.terminate()
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return report["exitCode"]


if __name__ == "__main__": raise SystemExit(main())
