import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright


BASE_URL = os.environ.get("AGENT_VISUAL_BASE_URL", "http://127.0.0.1:4175")
REPORT_DIR = Path(__file__).parent / "reports"
CASES = (("desktop", 1440, 900), ("tablet", 980, 720), ("mobile", 390, 844), ("small", 360, 640))


def main():
    report = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(headless=True)
        try:
            for name, width, height in CASES:
                page = browser.new_page(viewport={"width": width, "height": height})
                page.goto(f"{BASE_URL}/#/agent", wait_until="networkidle")
                page.wait_for_selector("[data-agent-workspace]")
                geometry = page.evaluate(
                    """
                    () => {
                      const root = document.querySelector('[data-agent-workspace]');
                      const workbench = root?.querySelector('.agent-workbench');
                      const prompt = root?.querySelector('.agent-prompt-panel');
                      const run = root?.querySelector('.agent-run-panel');
                      const nav = document.querySelector('.nav-links');
                      return {
                        viewport: [innerWidth, innerHeight],
                        documentScrollWidth: document.documentElement.scrollWidth,
                        bodyScrollWidth: document.body.scrollWidth,
                        rootWidth: root?.getBoundingClientRect().width,
                        workbenchWidth: workbench?.getBoundingClientRect().width,
                        columns: workbench ? getComputedStyle(workbench).gridTemplateColumns : '',
                        promptHeight: prompt?.getBoundingClientRect().height,
                        runHeight: run?.getBoundingClientRect().height,
                        navScrollWidth: nav?.scrollWidth,
                        navClientWidth: nav?.clientWidth
                      };
                    }
                    """
                )
                if geometry["documentScrollWidth"] > width + 1 or geometry["bodyScrollWidth"] > width + 1:
                    raise AssertionError(
                        f"{name}: page horizontal overflow "
                        f"{geometry['documentScrollWidth']}/{geometry['bodyScrollWidth']} > {width}"
                    )
                if name == "mobile":
                    page.locator(".agent-file-row").nth(1).click()
                    page.locator(".agent-run-button").click()
                    page.wait_for_timeout(550)
                    geometry["interaction"] = {
                        "selected": page.locator(".agent-file-row").nth(1).evaluate(
                            "element => element.classList.contains('is-selected')"
                        ),
                        "status": page.locator(".agent-page-header__status").inner_text(),
                        "output": page.locator(".agent-output-panel p").inner_text(),
                    }
                page.evaluate("window.scrollTo(0, 0)")
                page.screenshot(path=str(REPORT_DIR / f"agent-{name}-current.png"), full_page=True)
                report.append({"name": name, "geometry": geometry})
                page.close()
        finally:
            browser.close()
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
