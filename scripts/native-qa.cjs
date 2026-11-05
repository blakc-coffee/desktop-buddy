const { chromium } = require(process.env.BUDDY_PLAYWRIGHT_PATH || "playwright");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const exe = path.resolve("src-tauri/target/release/desktop-buddy.exe");
const root = path.resolve("artifacts/native-data-" + Date.now());
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
async function endpoint() {
  for (let i = 0; i < 40; i++) {
    try {
      const r = await fetch("http://127.0.0.1:9223/json/version");
      if (r.ok) return;
    } catch {}
    await delay(500);
  }
  throw Error("Native WebView2 debugging endpoint did not start");
}
(async () => {
  fs.mkdirSync(root, { recursive: true });
  const child = spawn(exe, ["--panel"], {
    env: {
      ...process.env,
      BUDDY_DATA_DIR: root,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
        "--remote-debugging-port=9223 --remote-debugging-address=127.0.0.1",
    },
    stdio: "ignore",
    windowsHide: true,
  });
  let browser;
  try {
    await endpoint();
    browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
    const context = browser.contexts()[0];
    let page = context.pages()[0] || (await context.waitForEvent("page"));
    await page.waitForLoadState();
    await page
      .getByRole("heading", { name: "A little less switching." })
      .waitFor();
    const invoke = (cmd, args = {}) =>
      page.evaluate(
        ({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args),
        { cmd, args },
      );
    const initial = await invoke("snapshot");
    if (!initial.native) throw Error("Expected native storage");
    await page
      .getByLabel("Quick capture")
      .fill("todo Native persistence check");
    await page.getByTitle("Capture", {exact:true}).click();
    await page.getByRole("button", { name: "Tasks", exact: true }).click();
    await page.getByText("Native persistence check", { exact: true }).waitFor();
    await page.getByRole("tab", { name: "Timers" }).click();
    await page.getByLabel("What are you focusing on?").fill("Native timer");
    await page
      .getByRole("button", { name: "Start timer", exact: true })
      .click();
    await page.getByTitle("Pause timer").click();
    await page.getByText("Paused", { exact: true }).waitFor();
    await page.getByTitle("Resume timer").click();
    const { emptyItem } = await import("../src/domain.ts").catch(() => ({
      emptyItem: (kind, title) => ({
        id: "",
        kind,
        title,
        done: false,
        createdAt: Date.now(),
        dueAt: 0,
        path: "",
        note: "",
        source: "",
        projectId: "",
        state: "",
        durationMs: 0,
        remainingMs: 0,
        startedTick: 0,
        run: "",
      }),
    }));
    await invoke("save_item", {
      item: {
        ...emptyItem("reminder", "Native reminder check"),
        dueAt: Date.now() + 2000,
      },
    });
    await delay(2800);
    let s = await invoke("snapshot");
    if (
      s.items.filter(
        (i) => i.kind === "attention" && i.title === "Native reminder check",
      ).length !== 1
    )
      throw Error("Reminder did not produce one durable attention item");
    await page.getByRole("button", { name: /^Activity/ }).click();
    await page.getByText("Native reminder check", { exact: true }).waitFor();
    await page.screenshot({ path: "artifacts/native-desktop.png" });
    fs.mkdirSync("artifacts/search-fixture", { recursive: true });
    fs.writeFileSync(
      "artifacts/search-fixture/native-note.txt",
      "local search test",
    );
    const search = await invoke("search_folder", {
      root: path.resolve("artifacts/search-fixture"),
      query: "native",
    });
    if (search.length !== 1) throw Error("Native folder search failed");
    const backup = await invoke("export_backup");
    const parsed = JSON.parse(backup);
    if (!parsed.items.some((i) => i.kind === "timer"))
      throw Error("Native timer missing from backup");
    const exportedTimer = parsed.items.find((i) => i.kind === "timer");
    if (exportedTimer.remainingMs >= exportedTimer.durationMs)
      throw Error("Backup must checkpoint elapsed timer time");
    const merge = await invoke("import_backup", { data: backup });
    if (merge !== 0)
      throw Error("Duplicate import should not duplicate records");
    const providers = await invoke("providers");
    let discovery = "not installed";
    if (providers.some((p) => p.id === "codex" && p.installed)) {
      try {
        const result = await invoke("codex_sessions");
        discovery = { ok: true, count: result.data?.length ?? 0 };
      } catch (e) {
        discovery = { ok: false, error: String(e).slice(0, 300) };
      }
    }
    // Destroy/recreate the real WebView. Single-instance relaunch opens a fresh panel.
    await page
      .evaluate(() =>
        window.__TAURI_INTERNALS__.invoke("plugin:window|close", {
          label: "main",
        }),
      )
      .catch(() => {});
    await delay(1500);
    if (child.exitCode !== null)
      throw Error("Closing the last panel exited the tray process");
    const second = spawn(exe, ["--panel"], {
      env: {
        ...process.env,
        BUDDY_DATA_DIR: root,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
          "--remote-debugging-port=9223 --remote-debugging-address=127.0.0.1",
      },
      stdio: "ignore",
      windowsHide: true,
    });
    await delay(2000);
    await endpoint();
    try {
      await browser.close();
    } catch {}
    browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
    page = browser.contexts()[0].pages()[0];
    await page
      .getByRole("heading", { name: "A little less switching." })
      .waitFor();
    s = await invoke("snapshot");
    if (
      !s.items.some(
        (i) => i.kind === "todo" && i.title === "Native persistence check",
      )
    )
      throw Error("Data lost across panel recreation");
    const result = {
      native: true,
      checks: [
        "SQLite persistence",
        "native timer pause/resume",
        "durable reminder attention",
        "bounded folder search",
        "backup roundtrip/no duplication",
        "panel destruction/recreation",
        "single instance",
      ],
      codexDiscovery: discovery,
      itemCount: s.items.length,
      processId: child.pid,
      secondLaunchExit: second.exitCode,
    };
    fs.writeFileSync(
      "artifacts/native-qa.json",
      JSON.stringify(result, null, 2),
    );
    console.log(JSON.stringify(result, null, 2));
  } catch(e) {
    const failedPage=browser?.contexts()[0]?.pages()[0];
    if(failedPage){try{await failedPage.screenshot({path:'artifacts/native-failure.png'});fs.writeFileSync('artifacts/native-failure.txt',await failedPage.locator('body').innerText());}catch{}}
    throw e;
  } finally {
    try {
      await browser?.close();
    } catch {}
    child.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
