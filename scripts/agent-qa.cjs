// Native E2E with a PATH-injected protocol fixture. No model calls.
const { chromium } = require(process.env.BUDDY_PLAYWRIGHT_PATH || "playwright");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const root = path.resolve("artifacts/agent-data-" + Date.now());
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
(async () => {
  fs.mkdirSync(root, { recursive: true });
  const child = spawn(
    path.resolve("src-tauri/target/release/desktop-buddy.exe"),
    ["--panel"],
    {
      env: {
        ...process.env,
        PATH: path.resolve("artifacts/mock-bin") + ";" + process.env.PATH,
        BUDDY_DATA_DIR: root,
        BUDDY_FIXTURE_LOG: path.join(root, "protocol.log"),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
          "--remote-debugging-port=9224 --remote-debugging-address=127.0.0.1",
      },
      windowsHide: true,
      stdio: "ignore",
    },
  );
  let browser, page;
  const results = [];
  try {
    for (let i = 0; i < 50; i++) {
      try {
        if ((await fetch("http://127.0.0.1:9224/json/version")).ok) break;
      } catch {}
      await delay(400);
    }
    browser = await chromium.connectOverCDP("http://127.0.0.1:9224");
    page = browser.contexts()[0].pages()[0];
    await page
      .getByRole("heading", { name: "A little less switching." })
      .waitFor();
    const invoke = (cmd, args = {}) =>
      page.evaluate(
        ({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args),
        { cmd, args },
      );
    const item = {
      id: "",
      kind: "project",
      title: "Protocol fixture workspace",
      done: false,
      createdAt: Date.now(),
      dueAt: 0,
      path: root,
      note: "QA only",
      source: "",
      projectId: "",
      state: "",
      durationMs: 0,
      remainingMs: 0,
      startedTick: 0,
      run: "",
    };
    await invoke("save_item", { item });
    const snapshot = await invoke("snapshot");
    const project = snapshot.items.find((i) => i.kind === "project");
    await page.getByRole("button", { name: /^Activity/ }).click();
    await page.getByRole("button", { name: "New task", exact: true }).click();
    await page.getByRole("button", { name: "Load available models" }).click();
    await page.getByLabel("Agent model").selectOption("fixture-model");
    await page.getByLabel("Agent workspace").selectOption(project.id);
    await page.getByLabel("Codex request").fill("Approval fixture");
    await page
      .getByRole("button", { name: "Send to Codex", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Approve once", exact: true })
      .waitFor();
    let task = (await invoke("agent_snapshot"))[0];
    const staleId = task.pending[0].id;
    await page
      .getByRole("button", { name: "Approve once", exact: true })
      .click();
    await page.getByLabel("Choose a fixture result").selectOption("B");
    const waiting = (await invoke("agent_snapshot")).find(
      (t) => t.id === task.id,
    );
    if (waiting.pending.length !== 1 || waiting.pending[0].id === staleId)
      throw Error("Answered approval reappeared");
    await page
      .getByRole("button", { name: "Send answer", exact: true })
      .click();
    async function waitState(id, state, connected) {
      for (let n = 0; n < 50; n++) {
        const t = (await invoke("agent_snapshot")).find((t) => t.id === id);
        if (
          t &&
          t.state === state &&
          (connected === undefined || t.connected === connected)
        )
          return t;
        await delay(100);
      }
      throw Error("Expected state " + state + " for " + id);
    }
    task = await waitState(task.id, "completed", true);
    if (!task.output.includes("No model") || task.output.includes("LATE"))
      throw Error("Receipt or terminal guard failed");
    let rejected = false;
    try {
      await invoke("agent_respond", {
        taskId: task.id,
        interactionId: staleId,
        decision: "accept",
        answers: {},
      });
    } catch {
      rejected = true;
    }
    if (!rejected) throw Error("Terminal approval accepted");
    rejected = false;
    try {
      await invoke("agent_cancel", { taskId: task.id, stopConnection: false });
    } catch {
      rejected = true;
    }
    if (
      !rejected ||
      (await invoke("agent_snapshot")).find((t) => t.id === task.id).state !==
        "completed"
    )
      throw Error("Cancel overwrote completion");
    await page
      .locator(".agent-task details")
      .first()
      .locator("summary")
      .click();
    await page.getByLabel("Follow-up todo").fill("Review fixture receipt");
    await page
      .getByRole("button", { name: "Create todo", exact: true })
      .click();
    const saved = await invoke("snapshot");
    if (
      !saved.items.some(
        (i) => i.kind === "todo" && i.title === "Review fixture receipt",
      )
    )
      throw Error("Followup not saved");
    await page.screenshot({ path: "artifacts/agent-desktop.png" });
    await invoke("agent_cancel", { taskId: task.id, stopConnection: true });
    await waitState(task.id, "completed", false);
    results.push(
      "model catalog, explicit submission, approve once, choice answer, final receipt, stale-answer rejection, late-event guard, followup todo, explicit connection close",
    );
    const start = (prompt) =>
      invoke("agent_start", {
        projectId: project.id,
        prompt,
        model: "fixture-model",
        writeAccess: false,
        acknowledgeUnknown: false,
      });
    const cancelId = await start("Cancel fixture");
    await waitState(cancelId, "running");
    await invoke("agent_cancel", { taskId: cancelId, stopConnection: false });
    await waitState(cancelId, "cancelled", true);
    await invoke("agent_cancel", { taskId: cancelId, stopConnection: true });
    await waitState(cancelId, "cancelled", false);
    results.push("interrupt waits for authoritative interrupted result");
    const dropId = await start("Drop fixture");
    await waitState(dropId, "unknown", false);
    rejected = false;
    try {
      await start("Drop fixture");
    } catch {
      rejected = true;
    }
    if (!rejected) throw Error("Unknown-outcome duplicate was accepted");
    results.push(
      "disconnect persists unknown; identical resubmission needs acknowledgment",
    );
    const backup = JSON.parse(await invoke("export_backup"));
    if (backup.version !== 2 || backup.agentTasks.length !== 3)
      throw Error("Managed receipts missing from backup");
    results.push("v2 backup includes managed receipts");
    const restored = {
      ...task,
      id: "restored-active-fixture",
      state: "running",
      connected: true,
    };
    await invoke("import_backup", {
      data: JSON.stringify({ version: 2, items: [], agentTasks: [restored] }),
    });
    const imported = (await invoke("agent_snapshot")).find(
      (t) => t.id === restored.id,
    );
    if (
      imported.state !== "unknown" ||
      imported.connected ||
      imported.pending.length
    )
      throw Error("Active backup replayed or retained decisions");
    results.push(
      "active backup restores unknown with no connection or decisions",
    );
    const startupDir = path.join(root, "startup-delay");
    fs.mkdirSync(startupDir);
    await invoke("save_item", {
      item: {
        ...item,
        title: "Startup cancellation fixture",
        path: startupDir,
      },
    });
    const startupProject = (await invoke("snapshot")).items.find(
      (i) => i.path === startupDir,
    );
    const startupId = await invoke("agent_start", {
      projectId: startupProject.id,
      prompt: "Never submit startup fixture",
      model: "fixture-model",
      writeAccess: false,
      acknowledgeUnknown: false,
    });
    for(let n=0;n<30;n++) {
      if(fs.readFileSync(path.join(root,'protocol.log'),'utf8').includes('startup-delay')) break;
      await delay(50);
    }
    await invoke("agent_cancel", { taskId: startupId, stopConnection: true });
    const cancelled = await waitState(startupId, "cancelled", false);
    if (cancelled.delivery !== "not_sent")
      throw Error("Startup cancellation sent a turn");
    const protocol = fs.readFileSync(path.join(root, "protocol.log"), "utf8");
    if (protocol.includes("Never submit startup fixture"))
      throw Error("Cancelled startup prompt reached fixture");
    results.push("startup stop persists cancelled before prompt delivery");
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.setViewportSize({ width: 390, height: 844 });
    if (
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      )
    )
      throw Error("Narrow viewport overflow");
    await page.screenshot({ path: "artifacts/agent-narrow.png" });
    if (errors.length) throw Error(errors.join("\n"));
    fs.writeFileSync(
      "artifacts/agent-qa.json",
      JSON.stringify(
        {
          fixtureOnly: true,
          noModelCalls: true,
          passed: results,
          dataDirectory: root,
        },
        null,
        2,
      ),
    );
    console.log(JSON.stringify({ passed: results }, null, 2));
  } catch (e) {
    if (page)
      await page
        .screenshot({ path: "artifacts/agent-failure.png" })
        .catch(() => {});
    throw e;
  } finally {
    if (browser) await browser.close().catch(() => {});
    child.kill();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
