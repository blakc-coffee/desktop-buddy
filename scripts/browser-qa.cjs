const { chromium } = require(process.env.BUDDY_PLAYWRIGHT_PATH || "playwright");
const fs = require("node:fs");
(async () => {
  const browser = await chromium.launch({ channel: "msedge", headless: true });
  const context = await browser.newContext({
    viewport: { width: 1100, height: 820 },
    acceptDownloads: true,
  });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(process.env.BUDDY_QA_URL || "http://127.0.0.1:1421");
  await page
    .getByRole("heading", { name: "A little less switching." })
    .waitFor();
  fs.mkdirSync("artifacts", { recursive: true });
  await page.screenshot({ path: "artifacts/desktop.png", fullPage: true });
  await page
    .getByLabel("Quick capture")
    .fill("todo Verify persistent local tasks");
  await page.getByLabel("Quick capture").press("Enter");
  await page.getByRole("button", { name: "Tasks", exact: true }).click();
  await page
    .getByText("Verify persistent local tasks", { exact: true })
    .waitFor();
  await page.reload();
  await page.getByRole("button", { name: "Tasks", exact: true }).click();
  await page
    .getByText("Verify persistent local tasks", { exact: true })
    .waitFor();
  await page
    .getByRole("checkbox", { name: "Complete Verify persistent local tasks" })
    .check();
  await page.getByRole("tab", { name: "Timers" }).click();
  await page
    .getByLabel("What are you focusing on?")
    .fill("Check timer recovery");
  await page.getByRole("button", { name: "Start timer", exact: true }).click();
  await page.getByTitle("Pause timer").click();
  await page.getByText("Paused", { exact: true }).waitFor();
  await page.getByTitle("Resume timer").click();
  await page.getByText("Focus in progress", { exact: true }).waitFor();
  await page
    .getByLabel("Quick capture")
    .fill("remind tomorrow 10:00 Review integration");
  await page.getByLabel("Quick capture").press("Enter");
  await page.getByRole("button", { name: "Add reminder" }).waitFor();
  if (await page.getByText("Review integration", { exact: true }).count())
    throw Error("Quick capture reminder saved before confirmation");
  await page.getByRole("button", { name: "Add reminder" }).click();
  await page.getByText("Review integration", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Projects", exact: true }).click();
  await page.getByLabel("Project name").fill("Integration work");
  await page.getByLabel("Folder path").fill("C:\\Projects\\buddy");
  await page
    .getByLabel("Where should you pick up?")
    .fill("Confirm source events before enabling controls.");
  await page.getByRole("button", { name: "Save project", exact: true }).click();
  await page
    .getByText("Confirm source events before enabling controls.", {
      exact: true,
    })
    .waitFor();
  await page.getByRole("button", { name: "Edit next step" }).click();
  await page
    .getByLabel("Quick capture")
    .fill("remind tomorrow 11:00 New reminder while editing");
  await page.getByLabel("Quick capture").press("Enter");
  await page.getByRole("button", { name: "Add reminder" }).click();
  await page.getByText("New reminder while editing", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Projects", exact: true }).click();
  await page
    .getByRole("heading", { name: "Integration work", exact: true })
    .waitFor();
  await page.screenshot({ path: "artifacts/projects.png", fullPage: true });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export backup" }).click();
  const d = await download;
  await d.saveAs("artifacts/qa-backup.json");
  const backup = JSON.parse(
    fs.readFileSync("artifacts/qa-backup.json", "utf8"),
  );
  if (!backup.items.some((i) => i.kind === "project"))
    throw Error("Project missing from exported backup");
  await page.getByRole("combobox").selectOption("light");
  await page.screenshot({ path: "artifacts/light.png", fullPage: true });
  await page.getByRole("combobox").selectOption("dark");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Tasks", exact: true }).click();
  await page.screenshot({ path: "artifacts/compact.png", fullPage: true });
  await page.setViewportSize({ width: 1100, height: 820 });
  await page.getByRole("tab", { name: "Reminders" }).click();
  await page.screenshot({ path: "artifacts/reminders.png", fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  if (overflow) throw Error("Compact layout overflows horizontally");
  await page.keyboard.press("Control+k");
  if (
    !(await page
      .getByLabel("Quick capture")
      .evaluate((el) => el === document.activeElement))
  )
    throw Error("Keyboard capture shortcut failed");
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    JSON.stringify(
      {
        checks: [
          "capture",
          "todo reload persistence",
          "todo completion",
          "timer pause/resume",
          "reminder confirmation",
          "project resume",
          "backup contents",
          "theme",
          "390px overflow",
          "keyboard shortcut",
        ],
        pageErrors: errors,
        backupItems: backup.items.length,
      },
      null,
      2,
    ),
  );
  await browser.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
