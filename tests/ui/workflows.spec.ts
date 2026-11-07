import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installFixture } from "./fixture";
async function go(page: Page, label: string) {
  const toggle = page.getByRole("button", { name: /^Tools/ });
  if (
    (await toggle.isVisible()) &&
    (await toggle.getAttribute("aria-expanded")) === "false"
  )
    await toggle.click();
  await page
    .locator(".nav-item")
    .filter({ hasText: new RegExp(`^${label}`) })
    .click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(label);
}
for (const theme of ["dark", "light"])
  for (const width of [1100, 380]) {
    test(`all tools are accessible, ${theme}, ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await installFixture(page, theme);
      await page.goto("/");
      await expect(
        page.getByText("Your focus session finished", { exact: true }),
      ).toBeVisible();
      for (const label of [
        "Overview",
        "Tasks",
        "Projects",
        "Shelf",
        "Codex",
        "Browser helper",
        "Music",
        "Settings",
      ]) {
        await go(page, label);
        await expect(page.locator(".loading")).toHaveCount(0);
        expect(
          await page.evaluate(
            () => document.documentElement.scrollWidth > innerWidth,
          ),
        ).toBe(false);
        const results = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
          .analyze();
        expect(
          results.violations.map(({ id, nodes }) => ({
            id,
            targets: nodes.map((node) => node.target),
          })),
        ).toEqual([]);
      }
      const calls = await page.evaluate(() =>
        (window as any).fixtureCalls.map((call: any) => call.command),
      );
      expect(calls).not.toContain("agent_start");
      expect(calls).not.toContain("codex_models");
      expect(calls).not.toContain("browser_generate");
    });
  }
test("checklist edits, cancellation, completion and completed disclosure", async ({
  page,
}) => {
  await installFixture(page);
  await page.goto("/");
  await go(page, "Tasks");
  const title = page.getByRole("textbox", {
    name: "Edit Review the browser-helper setup",
    exact: true,
  });
  await title.fill("Unsaved replacement");
  await title.press("Escape");
  await expect(title).toHaveValue("Review the browser-helper setup");
  await title.fill("Review the release checklist");
  await title.press("Enter");
  // Completion moves the row into a closed disclosure, so assert the resulting
  // list state instead of Playwright's in-place checkbox postcondition.
  await page
    .getByRole("checkbox", {
      name: "Complete Review the release checklist",
      exact: true,
    })
    .click();
  await expect(
    page.getByRole("checkbox", {
      name: "Complete Review the release checklist",
      exact: true,
    }),
  ).toBeHidden();
  await page.locator(".completed-todos > summary").click();
  await expect(
    page.getByRole("checkbox", {
      name: "Complete Review the release checklist",
      exact: true,
    }),
  ).toBeChecked();
});
test("drafts survive navigation without leaking between forms", async ({
  page,
}) => {
  await installFixture(page);
  await page.goto("/");
  await go(page, "Projects");
  await page
    .locator(".compose-panel > summary")
    .filter({ hasText: "Add a project" })
    .click();
  await page
    .getByRole("textbox", { name: "Project name", exact: true })
    .fill("Unsubmitted project");
  await page
    .getByRole("textbox", { name: "Your next step", exact: true })
    .fill("Draft next step");
  await go(page, "Tasks");
  await expect(
    page.getByRole("textbox", { name: "Title", exact: true }),
  ).toHaveValue("");
  await go(page, "Projects");
  await expect(
    page.getByRole("textbox", { name: "Project name", exact: true }),
  ).toHaveValue("Unsubmitted project");
  await expect(
    page.getByRole("textbox", { name: "Your next step", exact: true }),
  ).toHaveValue("Draft next step");
  await go(page, "Codex");
  await page
    .getByRole("button", { name: "Ask about files", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Codex question", exact: true })
    .fill("Draft question");
  await go(page, "Overview");
  await go(page, "Codex");
  await expect(
    page.getByRole("textbox", { name: "Codex question", exact: true }),
  ).toHaveValue("Draft question");
});
test("native destinations focus the matching workspace and acknowledge once", async ({
  page,
}) => {
  await installFixture(page);
  await page.goto("/");
  await page
    .getByText("Your focus session finished", { exact: true })
    .waitFor();
  await page.evaluate(() =>
    (window as any).fixtureEmit("panel-navigation", {
      id: 1,
      destination: "ask",
    }),
  );
  await expect(
    page.getByRole("textbox", { name: "Codex question", exact: true }),
  ).toBeFocused();
  await page.evaluate(() =>
    (window as any).fixtureEmit("panel-navigation", {
      id: 2,
      destination: "resume",
    }),
  );
  await expect(
    page.getByRole("heading", { name: "Pick up a project", exact: true }),
  ).toBeFocused();
  await page.evaluate(() => {
    (window as any).fixtureEmit("panel-navigation", {
      id: 4,
      destination: "timers",
    });
    (window as any).fixtureEmit("panel-navigation", {
      id: 3,
      destination: "reminders",
    });
  });
  await expect(
    page.getByRole("button", { name: "Timers", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect
    .poll(() => page.evaluate(() => (window as any).fixtureAcks))
    .toEqual([1, 2, 4]);
});
test("keyboard can skip chrome and edit a checklist", async ({ page }) => {
  await installFixture(page);
  await page.goto("/");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Skip to content" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
  await go(page, "Tasks");
  await page.getByRole("button", { name: "Timers", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("button", { name: "Timers", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
});
test("pending decisions remain discoverable and receipt links cross workspaces", async ({
  page,
}) => {
  await installFixture(page);
  await page.goto("/");
  await page
    .getByText("Your focus session finished", { exact: true })
    .waitFor();
  await page.evaluate(() => {
    const root = window as any;
    const task = {
      id: "fixture-task",
      title: "Synthetic managed task",
      prompt: "Fixture request",
      context: "",
      projectId: "p1",
      workspace: "C:\\Projects\\Desktop Buddy",
      model: "fixture-model",
      writeAccess: false,
      state: "awaiting_approval",
      activity: "Review this reported operation",
      output: "",
      outputTruncated: false,
      sessionId: "fixture-session",
      turnId: "fixture-turn",
      delivery: "received",
      createdAt: Date.now(),
      updatedAt: Date.now(),
      completedAt: 0,
      pending: [
        {
          id: "fixture-decision",
          method: "item/commandExecution/requestApproval",
          title: "Synthetic decision",
          detail: "This is test data; no command will run.",
          canApprove: true,
          questions: [],
        },
      ],
      commands: [],
      files: [],
      seen: false,
      connected: true,
      cancelRequested: false,
    };
    root.fixtureTasks = [task];
    root.fixtureEmit("agent-changed", null);
    root.fixtureEmit("data-changed", null);
  });
  await page.getByRole("button", { name: "Review Codex", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Approve once", exact: true }),
  ).toBeVisible();
  await go(page, "Overview");
  await go(page, "Codex");
  await expect(
    page.getByRole("button", { name: "Approve once", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).fixtureCalls.some(
        (call: any) => call.command === "agent_respond",
      ),
    ),
  ).toBe(false);
  await page.evaluate(() => {
    const root = window as any;
    Object.assign(root.fixtureTasks[0], {
      state: "completed",
      connected: false,
      pending: [],
      output: "Synthetic completion receipt",
      completedAt: Date.now(),
    });
    root.fixtureEmit("agent-changed", null);
    root.fixtureEmit("data-changed", null);
  });
  await go(page, "Projects");
  await page
    .getByRole("button", { name: "View task details", exact: true })
    .click();
  await expect(page.locator("#agent-task-fixture-task")).toBeFocused();
  await expect(
    page.locator("#agent-task-fixture-task .receipt"),
  ).toHaveAttribute("open", "");
});
test("compact Todo stays editable and accessible", async ({ page }) => {
  await page.setViewportSize({ width: 300, height: 420 });
  await installFixture(page);
  await page.goto("/?surface=todo");
  await expect(
    page.getByRole("heading", { name: "Todo", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("checkbox", {
      name: "Complete Review the browser-helper setup",
      exact: true,
    }),
  ).toBeVisible();
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(results.violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
  ).toBe(false);
});

test("empty data, loading recovery and reduced-motion compact layout", async ({
  page,
}) => {
  await page.setViewportSize({ width: 550, height: 800 });
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await installFixture(page, "system", true);
  await page.addInitScript("window.fixtureSnapshotError = 100");
  await page.goto("/");
  await page
    .getByRole("button", { name: "Retry loading", exact: true })
    .waitFor();
  await page
    .getByRole("button", { name: "Retry loading", exact: true })
    .evaluate((button) => {
      // Clear the synthetic failure and retry atomically: background refreshes
      // otherwise recover first and remove this initial-load-only control.
      (window as any).fixtureSnapshotError = 0;
      (button as HTMLButtonElement).click();
    });
  await expect(page.locator(".loading")).toHaveCount(0);
  await go(page, "Tasks");
  await expect(
    page.getByRole("textbox", { name: "Title", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
  ).toBe(false);
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(results.violations).toEqual([]);
  expect(
    await page
      .locator(".nav-item")
      .first()
      .evaluate((el) => getComputedStyle(el).transitionDuration),
  ).toBe("0s");
});
