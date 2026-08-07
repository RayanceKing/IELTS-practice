#!/usr/bin/env python3
"""Truthful packaged Tauri 2 smoke gate.

This deliberately speaks the WebDriver HTTP protocol directly, so the gate
has no hidden Playwright/file:// fallback.  Set TAURI_APP_BINARY to a built
Tauri executable and ensure tauri-driver plus the platform native driver are
on PATH (or set TAURI_DRIVER/TAURI_NATIVE_DRIVER).
"""
from __future__ import annotations

import base64
import hashlib
import json
import math
import os
import shutil
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REPORT = ROOT / "developer/tests/e2e/reports/suite-practice-flow-report.json"
READING_SCREENSHOT = ROOT / "developer/tests/e2e/reports/reading-practice-current.png"
LIBRARY_SCREENSHOT = ROOT / "developer/tests/e2e/reports/library-current.png"
SETTINGS_SCREENSHOT = ROOT / "developer/tests/e2e/reports/settings-current.png"
HISTORY_SCREENSHOT = ROOT / "developer/tests/e2e/reports/history-current.png"
COMPOSE_SCREENSHOT = ROOT / "developer/tests/e2e/reports/compose-current.png"
TOPICS_SCREENSHOT = ROOT / "developer/tests/e2e/reports/topics-current.png"
AGENT_SCREENSHOT = ROOT / "developer/tests/e2e/reports/agent-current.png"
READING_P95_BUDGET_MS = 3000


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_value(*args: str) -> str | None:
    try:
        result = subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, text=True, timeout=10, check=False
        )
        value = result.stdout.strip()
        return value or None
    except (OSError, subprocess.SubprocessError):
        return None


def latest_shipping_source_mtime() -> float:
    roots = (
        ROOT / "apps/writing-vue/src",
        ROOT / "src-tauri/src",
        ROOT / "crates/ielts-domain/src",
        ROOT / "crates/ielts-db/src",
        ROOT / "dist/writing",
    )
    candidates = [
        ROOT / "src-tauri/tauri.conf.json",
        ROOT / "src-tauri/Cargo.toml",
        ROOT / "apps/writing-vue/package.json",
    ]
    for root in roots:
        if root.is_dir():
            candidates.extend(path for path in root.rglob("*") if path.is_file())
    return max((path.stat().st_mtime for path in candidates if path.is_file()), default=0.0)


def ensure_current_binary(app: Path, explicit: bool) -> tuple[Path, bool, str | None]:
    if explicit:
        return app, False, None
    stale = not app.is_file() or app.stat().st_mtime < latest_shipping_source_mtime()
    if not stale:
        return app, False, None
    completed = subprocess.run(
        ["cargo", "build", "--release", "-p", "ielts-practice-tauri"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=300,
        check=False,
    )
    detail = "\n".join(part.strip() for part in (completed.stdout, completed.stderr) if part.strip())
    if completed.returncode != 0:
        raise RuntimeError(f"current packaged binary build failed: {detail[-4000:]}")
    return ROOT / "target/release/ielts-practice-tauri.exe", True, detail[-1000:] or None


def binary_metadata(app: Path, tauri: str | None, native: str | None, build_performed: bool) -> dict:
    status = git_value("status", "--porcelain")
    return {
        "gitCommit": git_value("rev-parse", "HEAD"),
        "gitDirty": bool(status),
        "binaryPath": str(app.resolve()) if app.is_file() else str(app),
        "binarySha256": sha256_file(app) if app.is_file() else None,
        "binarySize": app.stat().st_size if app.is_file() else None,
        "binaryModifiedAt": datetime.fromtimestamp(app.stat().st_mtime, timezone.utc).isoformat() if app.is_file() else None,
        "buildPerformed": build_performed,
        "tauriDriverVersion": executable_version(tauri),
        "nativeDriverVersion": executable_version(native),
    }


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
        if result.returncode != 0:
            return None
        value = (result.stdout or result.stderr).strip()
        return value or None
    except (OSError, subprocess.SubprocessError):
        return None


def blocked(reason: str, missing: list[str]) -> int:
    report = {"schemaVersion": 2, "generatedAt": datetime.now(timezone.utc).isoformat(),
              "status": "blocked", "exitCode": 2, "target": "packaged-tauri-2",
              "reason": reason, "missingDependencies": missing,
              "checks": {"launch": "blocked", "vueRoutes": "blocked", "readingIpc": "blocked",
                         "uiRouteVisuals": "blocked",
                         "bundledResources": "blocked", "readingView": "blocked",
                         "readingPerformance": "blocked", "notesDialog": "blocked",
                         "readingSubmitBoundary": "blocked",
                         "backupPathBoundary": "blocked",
                         "updaterBoundary": "blocked",
                         "sqliteRestart": "blocked"}}
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
    def screenshot(self, path: Path):
        value = self.call("GET", f"/session/{self.sid}/screenshot")
        encoded = value.get("value", value)
        if not isinstance(encoded, str) or not encoded:
            raise RuntimeError(f"WebDriver screenshot failed: {value}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(base64.b64decode(encoded))
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


def wait_for_value(driver: Driver, source: str, timeout_seconds: int = 15):
    deadline = time.time() + timeout_seconds
    last = None
    while time.time() < deadline:
        last = driver.script(source)
        if last:
            return last
        time.sleep(0.1)
    raise RuntimeError(f"WebDriver condition timed out: {last}")


def wait_for_reading_view(driver: Driver, timeout_seconds: int = 30):
    deadline = time.time() + timeout_seconds
    last = None
    while time.time() < deadline:
        last = driver.script("""
            const error = document.querySelector('.inline-message-error');
            const workspace = document.querySelector('[data-practice-reading-page]');
            return {
              ready: !!workspace && workspace.textContent.trim().length > 0,
              error: error ? error.textContent.trim() : '',
              hash: location.hash
            };
        """)
        if isinstance(last, dict) and last.get("error"):
            raise RuntimeError(f"Reading view failed: {last}")
        if isinstance(last, dict) and last.get("ready"):
            return last
        time.sleep(0.1)
    raise RuntimeError(f"Reading view did not become ready: {last}")


def capture_route_visual(driver: Driver, route: str, selector: str, screenshot: Path) -> dict:
    driver.script("location.hash = arguments[0]; return location.hash", [route])
    wait_for_value(driver, f"""
        const root = document.querySelector({selector!r});
        const transitioning = document.querySelector('.page-enter-active, .page-enter-to, .page-leave-active, .page-leave-to');
        return location.hash === {route!r}
          && root
          && root.getBoundingClientRect().width > 0
          && root.getBoundingClientRect().height > 0
          && !transitioning;
    """)
    # Vue out-in transitions can remove their classes one frame before the
    # outgoing page is actually painted. Capture stable route pixels only.
    time.sleep(0.45)
    metrics = driver.script(f"""
        const root = document.querySelector({selector!r});
        const rect = root.getBoundingClientRect();
        const offenders = Array.from(document.body.querySelectorAll('*'))
          .map((element) => {{
            const value = element.getBoundingClientRect();
            const style = getComputedStyle(element);
            const className = typeof element.className === 'string'
              ? element.className
              : (element.className?.baseVal || '');
            return {{
              tag: element.tagName,
              id: element.id || '',
              className,
              left: value.left,
              right: value.right,
              width: value.width,
              display: style.display,
              position: style.position
            }};
          }})
          .filter((item) => item.display !== 'none' && item.width > 0 && (item.left < -2 || item.right > innerWidth + 2))
          .sort((left, right) => Math.max(right.right - innerWidth, -right.left) - Math.max(left.right - innerWidth, -left.left))
          .slice(0, 12);
        return {{
          width: rect.width,
          height: rect.height,
          scrollWidth: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
          viewportWidth: innerWidth,
          textLength: root.textContent.trim().length,
          offenders
        }};
    """)
    driver.screenshot(screenshot)
    if not isinstance(metrics, dict) or metrics.get("width", 0) < 320 or metrics.get("height", 0) < 120:
        raise RuntimeError(f"route {route} root is collapsed: {metrics}")
    if metrics.get("scrollWidth", 0) > metrics.get("viewportWidth", 0) + 24:
        raise RuntimeError(f"route {route} has unexpected horizontal overflow: {metrics}")
    return {**metrics, "screenshot": str(screenshot.resolve())}


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
    build_performed = False
    build_detail = None
    try:
        app, build_performed, build_detail = ensure_current_binary(app, bool(explicit_app))
    except Exception as exc:
        return blocked("current packaged executable could not be built", [str(exc)])
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
    metadata = binary_metadata(app, tauri, native, build_performed)
    if build_detail:
        metadata["buildDetail"] = build_detail
    try:
        for _ in range(30):
            try: driver.call("GET", "/status"); break
            except Exception: time.sleep(1)
        driver.create(str(app.resolve()))
        checks["launch"] = "passed"
        wait_for_vue(driver)
        for route in ("#/writing", "#/topics", "#/settings", "#/history", "#/agent", "#/"):
            reached = driver.script("location.hash=arguments[0]; return location.hash === arguments[0]", [route])
            if not reached: raise RuntimeError(f"Vue hash route failed: {route}")
        checks["vueRoutes"] = "passed"
        metadata["uiRouteVisuals"] = {
            "library": capture_route_visual(
                driver, "#/", "[data-practice-reading-home]", LIBRARY_SCREENSHOT
            ),
            "compose": capture_route_visual(
                driver, "#/writing", ".compose-page", COMPOSE_SCREENSHOT
            ),
            "topics": capture_route_visual(
                driver, "#/topics", ".topic-manage-page", TOPICS_SCREENSHOT
            ),
            "settings": capture_route_visual(
                driver, "#/settings", "[data-writing-settings]", SETTINGS_SCREENSHOT
            ),
            "history": capture_route_visual(
                driver, "#/history", ".history-page", HISTORY_SCREENSHOT
            ),
            "agent": capture_route_visual(
                driver, "#/agent", "[data-agent-workspace]", AGENT_SCREENSHOT
            ),
        }
        checks["uiRouteVisuals"] = "passed"
        result = driver.script("return window.__TAURI_INTERNALS__ ? window.__TAURI_INTERNALS__.invoke('reading_list_assets') : null")
        assets = (result or {}).get("data") if isinstance(result, dict) else None
        if not assets: raise RuntimeError("Tauri IPC bridge unavailable or reading_list_assets returned empty")
        checks["readingIpc"] = "passed"
        archive = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('reading_export_archive')"
        )
        archive_data = (archive or {}).get("data") if isinstance(archive, dict) else None
        if not isinstance(archive, dict) or not archive.get("ok") or not isinstance(archive_data, dict):
            raise RuntimeError(f"reading_export_archive failed: {archive}")
        if archive_data.get("schemaVersion") != "practice-history-archive.v2":
            raise RuntimeError(f"reading_export_archive returned a non-canonical schema: {archive_data}")
        rejected_archive = driver.script("""
            return window.__TAURI_INTERNALS__.invoke('reading_import_archive', {value: {
              activity: 'reading', schemaVersion: 'practice-history-archive.v2',
              exportedAt: '2026-01-01T00:00:00Z', count: 1, submissions: [{}]
            }})
        """)
        rejected_data = (rejected_archive or {}).get("data") if isinstance(rejected_archive, dict) else None
        if (not isinstance(rejected_archive, dict) or not rejected_archive.get("ok")
                or not isinstance(rejected_data, dict) or rejected_data.get("committed")
                or rejected_data.get("imported") != 0):
            raise RuntimeError(f"reading_import_archive did not fail closed: {rejected_archive}")
        checks["readingArchiveBoundary"] = "passed"
        retention = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('history_get_retention_policy')"
        )
        retention_data = (retention or {}).get("data") if isinstance(retention, dict) else None
        if not isinstance(retention, dict) or not retention.get("ok") or not isinstance(retention_data, dict):
            raise RuntimeError(f"history_get_retention_policy failed: {retention}")
        invalid_retention = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('history_set_retention_policy', {cmd: {maxTerminalAttempts: 51}})"
        )
        if not isinstance(invalid_retention, dict) or invalid_retention.get("ok"):
            raise RuntimeError(f"invalid retention policy was accepted: {invalid_retention}")
        checks["historyRetentionBoundary"] = "passed"
        asset_id = assets[0].get("id") or assets[0].get("assetId") or assets[0].get("asset_id")
        if not asset_id:
            raise RuntimeError(f"reading_list_assets returned an entry without an id: {assets[0]}")
        payload = driver.script("return window.__TAURI_INTERNALS__.invoke('reading_get_asset_payload', {assetId: arguments[0]})", [asset_id])
        payload_data = payload.get("data") if isinstance(payload, dict) else None
        if not isinstance(payload, dict) or not payload.get("ok") or not isinstance(payload_data, dict):
            raise RuntimeError(f"reading_get_asset_payload failed for {asset_id}: {payload}")
        if not isinstance(payload_data.get("asset"), dict) or "payload" not in payload_data:
            raise RuntimeError(f"reading payload is not canonical {{asset,payload}}: {payload_data}")
        if isinstance(payload_data.get("payload"), dict) and "asset" in payload_data["payload"] and "payload" in payload_data["payload"]:
            raise RuntimeError("reading payload is double wrapped")
        checks["bundledResources"] = "passed"
        navigation_samples = []
        encoded_asset_id = urllib.parse.quote(str(asset_id), safe="")
        for sample_index in range(5):
            driver.script("location.hash = '#/'; return true")
            wait_for_value(driver, "return !document.querySelector('[data-practice-reading-page]')")
            route = f"#/reading/{encoded_asset_id}?e2eSample={sample_index}"
            driver.script(
                "window.__e2eReadingStart = performance.now(); location.hash = arguments[0]; return true",
                [route],
            )
            wait_for_reading_view(driver)
            elapsed = driver.script("return performance.now() - window.__e2eReadingStart")
            navigation_samples.append(round(float(elapsed), 2))
        p95_index = max(0, math.ceil(len(navigation_samples) * 0.95) - 1)
        reading_p95_ms = sorted(navigation_samples)[p95_index]
        metadata["readingNavigationMs"] = navigation_samples
        metadata["readingP95Ms"] = reading_p95_ms
        metadata["readingP95BudgetMs"] = READING_P95_BUDGET_MS
        if reading_p95_ms > READING_P95_BUDGET_MS:
            raise RuntimeError(
                f"reading view P95 {reading_p95_ms}ms exceeds {READING_P95_BUDGET_MS}ms"
            )
        checks["readingView"] = "passed"
        checks["readingPerformance"] = "passed"
        layout = driver.script("""
            const rect = (selector) => {
              const element = document.querySelector(selector);
              if (!element) return null;
              const value = element.getBoundingClientRect();
              return {x: value.x, y: value.y, width: value.width, height: value.height};
            };
            const workspace = document.querySelector('[data-practice-reading-page]');
            const style = workspace ? getComputedStyle(workspace) : null;
            return {
              viewport: {width: innerWidth, height: innerHeight, devicePixelRatio},
              workspace: rect('[data-practice-reading-page]'),
              left: rect('#left'),
              right: rect('#right'),
              divider: rect('[data-practice-reading-page] > #reading-divider'),
              passageHtml: rect('#left .passage-html'),
              gridTemplateColumns: style?.gridTemplateColumns || '',
              display: style?.display || '',
              children: workspace ? Array.from(workspace.children).map((element) => {
                const childRect = element.getBoundingClientRect();
                const childStyle = getComputedStyle(element);
                return {
                  tag: element.tagName,
                  id: element.id,
                  className: element.className,
                  x: childRect.x,
                  y: childRect.y,
                  width: childRect.width,
                  height: childRect.height,
                  position: childStyle.position,
                  gridColumn: childStyle.gridColumn,
                  gridRow: childStyle.gridRow
                };
              }) : []
            };
        """)
        metadata["readingLayout"] = layout
        if not isinstance(layout, dict):
            raise RuntimeError(f"reading layout metrics unavailable: {layout}")
        for pane_name in ("left", "right", "passageHtml"):
            pane = layout.get(pane_name) or {}
            if float(pane.get("width") or 0) < 240:
                raise RuntimeError(f"reading {pane_name} collapsed: {layout}")
        driver.screenshot(READING_SCREENSHOT)
        metadata["readingScreenshot"] = str(READING_SCREENSHOT.resolve())
        driver.script("""
            const button = document.querySelector('#note-btn');
            button.focus();
            button.click();
            return document.activeElement?.id;
        """)
        wait_for_value(
            driver,
            "return document.activeElement?.tagName === 'TEXTAREA' && getComputedStyle(document.querySelector('#notes-panel')).display !== 'none'",
        )
        tab_target = driver.script("""
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {key: 'Tab', bubbles: true}));
            return document.activeElement?.id || '';
        """)
        if tab_target != "close-note":
            raise RuntimeError(f"notes dialog did not trap Tab: {tab_target}")
        driver.script("""
            document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {key: 'Escape', bubbles: true}));
            return true;
        """)
        wait_for_value(
            driver,
            "return getComputedStyle(document.querySelector('#notes-panel')).display === 'none' && document.activeElement?.id === 'note-btn'",
        )
        checks["notesDialog"] = "passed"
        negative = driver.script("""
            return window.__TAURI_INTERNALS__.invoke('reading_submit_attempt', {cmd: {
              attemptId: 'e2e-negative-submit', assetId: arguments[0], answers: {},
              markedQuestions: [], questionTimeline: [], idempotencyKey: 'e2e-negative-key',
              payload: {answerKey: {q1: 'forged'}}
            }}).then(value => ({resolved: true, value})).catch(error => ({resolved: false, error: String(error)}));
        """, [asset_id])
        if isinstance(negative, dict) and negative.get("resolved") and (negative.get("value") or {}).get("ok"):
            raise RuntimeError("reading_submit_attempt accepted forbidden client payload/answerKey")
        checks["readingSubmitBoundary"] = "passed"
        created_backup = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('create_backup', {appVersion: null})"
        )
        if not isinstance(created_backup, dict) or not created_backup.get("ok"):
            raise RuntimeError(f"create_backup failed: {created_backup}")
        backups = driver.script("return window.__TAURI_INTERNALS__.invoke('list_backups')")
        backup_items = (backups or {}).get("data", []) if isinstance(backups, dict) else []
        if not backup_items:
            raise RuntimeError(f"list_backups returned no authorized grants: {backups}")
        if any("path" in item or not item.get("grantId") for item in backup_items):
            raise RuntimeError(f"list_backups leaked raw callable paths: {backup_items}")
        preview = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('import_backup_path', {grantId: arguments[0], dryRun: true})",
            [backup_items[0]["grantId"]],
        )
        if not isinstance(preview, dict) or not preview.get("ok"):
            raise RuntimeError(f"authorized backup dry-run failed: {preview}")
        forged_grant = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('import_backup_path', {grantId: 'forged-e2e-grant', dryRun: true})"
        )
        if not isinstance(forged_grant, dict) or forged_grant.get("ok"):
            raise RuntimeError(f"forged backup path grant was not rejected: {forged_grant}")
        forged_error = forged_grant.get("error") or {}
        if forged_error.get("code") != "backup.path_grant":
            raise RuntimeError(f"forged backup rejection used the wrong boundary: {forged_grant}")
        checks["backupPathBoundary"] = "passed"
        updater_status = driver.script(
            "return window.__TAURI_INTERNALS__.invoke('check_for_updates')"
        )
        if not isinstance(updater_status, dict):
            raise RuntimeError(f"check_for_updates returned an invalid status: {updater_status}")
        if updater_status.get("configured") or updater_status.get("stage") != "unconfigured":
            raise RuntimeError(f"development updater did not fail closed: {updater_status}")
        restart_without_install = driver.script("""
            return window.__TAURI_INTERNALS__.invoke('restart_after_update')
              .then(value => ({resolved: true, value}))
              .catch(error => ({resolved: false, error: String(error)}));
        """)
        if not isinstance(restart_without_install, dict) or restart_without_install.get("resolved"):
            raise RuntimeError(f"restart_after_update bypassed install state: {restart_without_install}")
        checks["updaterBoundary"] = "passed"
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
        report = {"schemaVersion": 2, "generatedAt": datetime.now(timezone.utc).isoformat(),
                  "status": status, "exitCode": 0 if status == "passed" else 1,
                  "target": "packaged-tauri-2", "metadata": metadata, "checks": checks}
    except Exception as exc:
        report = {"schemaVersion": 2, "generatedAt": datetime.now(timezone.utc).isoformat(),
                  "status": "failed", "exitCode": 1, "target": "packaged-tauri-2",
                  "metadata": metadata, "checks": checks, "error": str(exc)}
    finally:
        driver.close(); proc.terminate()
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return report["exitCode"]


if __name__ == "__main__": raise SystemExit(main())
