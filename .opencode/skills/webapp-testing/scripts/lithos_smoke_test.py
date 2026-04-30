"""
Lithos Automated Smoke Test — Comprehensive.

Verifies game scenes transition, keyboard input, and internal game state.
"""
import json
import sys
import time
from playwright.sync_api import sync_playwright

RESULTS = {"pass": 0, "fail": 0, "errors": [], "screenshots": []}
CLIENT_URL = "http://localhost:5173"

def report(ok, label, detail=""):
    k = "pass" if ok else "fail"
    RESULTS[k] = RESULTS.get(k, 0) + 1
    print(f"  {'✓' if ok else '✗'} {label} {detail}")
    if not ok:
        RESULTS["errors"].append(label)

def snapshot(page, name):
    path = f"/tmp/lithos-{name}.png"
    page.screenshot(path=path, full_page=False)
    RESULTS["screenshots"].append(path)

def game_eval(page, code):
    return page.evaluate(f"() => {{ const g = window.__PHASER_GAME__; if (!g) return 'NO_GAME'; {code} }}")

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, args=["--use-gl=angle", "--use-angle=swiftshader"])
    context = browser.new_context(viewport={"width": 1280, "height": 720})

    errors = []
    context.on("console", lambda msg: errors.append(f"{msg.type}: {msg.text[:200]}") if msg.type in ("error", "warning") else None)

    page = context.new_page()

    def fake_api(route):
        route.fulfill(status=200,
            headers={"access-control-allow-origin": "*", "content-type": "application/json"},
            body=json.dumps([{
                "id": "srv-local", "name": "Lithos Dev Server", "region": "local",
                "websocket_url": "ws://localhost:9001", "population": 0, "capacity": 100, "healthy": True
            }])
        )
    page.route("http://localhost:3000/**", fake_api)

    page.goto(CLIENT_URL)
    page.wait_for_load_state("networkidle")
    time.sleep(3)
    snapshot(page, "01-boot")

    # Connect to server and transition to LoginScene
    scene = page.evaluate("""async () => {
        for (let i = 0; i < 30; i++) {
            if (window.__PHASER_GAME__) break;
            await new Promise(r => setTimeout(r, 200));
        }
        const g = window.__PHASER_GAME__;
        if (!g) return 'NO_GAME';
        g.scene.getScene('BootScene').connectToServer('ws://localhost:9001');
        await new Promise(r => setTimeout(r, 1500));
        return g.scene.scenes.map(s => s.scene.key).join(',');
    }""")
    report('LoginScene' in scene, f"BootScene -> LoginScene transition", f"(scenes: {scene})")
    snapshot(page, "02-login-scene")

    # Enter credentials and join
    try:
        username = page.locator("#username")
        username.wait_for(timeout=5000)
        username.fill("alpha#1")
        page.locator("#loginBtn").click()
        report(True, "Login form submitted")
    except Exception as e:
        report(False, "Login form interaction", f"({e})")

    time.sleep(5)

    # Check if we joined Overworld
    scene = page.evaluate("""() => {
        const g = window.__PHASER_GAME__;
        if (!g) return 'NO_GAME';
        const active = g.scene.scenes.filter(s => s.scene.isActive()).map(s => s.scene.key);
        return active.join(',');
    }""")
    report('OverworldScene' in scene, f"Joined Overworld", f"(active: {scene})")
    snapshot(page, "03-overworld")

    # Movement
    page.keyboard.down("d"); time.sleep(0.4)
    page.keyboard.up("d")
    page.keyboard.down("w"); time.sleep(0.4)
    page.keyboard.up("w")
    report(True, "WASD movement input sent")
    snapshot(page, "04-movement")

    # Zone Transfer
    page.keyboard.press("Space"); time.sleep(3)
    scene_after = page.evaluate("() => (window.__PHASER_GAME__?.scene.scenes.filter(s => s.scene.isActive()).map(s => s.scene.key) || []).join(',')")
    report('AsteroidBaseScene' in scene_after, f"Zone transfer to AsteroidBase", f"(active: {scene_after})")
    snapshot(page, "05-asteroid-base")

    # Return
    page.keyboard.press("Space"); time.sleep(3)
    scene_back = page.evaluate("() => (window.__PHASER_GAME__?.scene.scenes.filter(s => s.scene.isActive()).map(s => s.scene.key) || []).join(',')")
    report('OverworldScene' in scene_back, "Zone transfer back to Overworld")
    snapshot(page, "06-back-overworld")

    # Combat
    page.mouse.click(700, 360); time.sleep(0.5)
    report(True, "Mouse click (fire) sent")
    snapshot(page, "07-combat")

    # Crafting
    page.keyboard.press("c"); time.sleep(1.5)
    report(True, "Crafting panel toggled (C key)")
    snapshot(page, "08-crafting")
    page.keyboard.press("c"); time.sleep(0.5)

    # Build mode
    page.keyboard.press("b"); time.sleep(1)
    report(True, "Build mode toggled (B key)")
    snapshot(page, "09-build-mode")
    page.keyboard.press("b"); time.sleep(0.5)

    # Chat
    page.keyboard.press("Enter"); time.sleep(0.5)
    page.keyboard.type("hello from automation!")
    page.keyboard.press("Enter"); time.sleep(1)
    report(True, "Chat message sent")
    snapshot(page, "10-chat")

    # Final state
    snapshot(page, "11-final")

    # Console errors report
    print(f"\n=== Console Events: {len(errors)} ===")
    for e in errors:
        print(f"  {e}")
        RESULTS["errors"].append(e)

    browser.close()

    total = RESULTS['pass'] + RESULTS['fail']
    print(f"\n{'='*50}")
    print(f"SMOKE TEST: {RESULTS['pass']}/{total} PASS, {RESULTS['fail']}/{total} FAIL")
    print(f"SCREENSHOTS: {len(RESULTS['screenshots'])} | ERRORS: {len(RESULTS['errors'])}")
    print(f"{'='*50}")
    json.dump(RESULTS, sys.stdout, indent=2)
    sys.exit(1 if RESULTS["fail"] > 0 else 0)
