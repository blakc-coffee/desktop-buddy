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
        "Shelf",
        "Screen helper",
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
test.skip("drafts survive navigation without leaking between forms", async ({
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
  await expect(page.getByRole("heading", {name:"Settings",exact:true})).toBeVisible();
  await page.evaluate(() => (window as any).fixtureEmit("panel-navigation", {id:2,destination:"resume"}));
  await expect(page.getByRole("heading", {name:"Settings",exact:true})).toBeVisible();
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
test.skip("pending decisions remain discoverable and receipt links cross workspaces", async ({
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

test("screen commands capture and explain or type once without browser connection", async ({
  page,
}) => {
  await installFixture(page);
  await page.goto("/");
  await page
    .getByText("Your focus session finished", { exact: true })
    .waitFor();
  await page.evaluate(() =>
    (window as any).fixtureEmit("panel-navigation", {
      id: 10,
      destination: "screenExplain",
    }),
  );
  await expect(
    page.getByRole("img", {
      name: "Captured display to review before explaining",
    }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Load and select a model once to continue this command. Buddy remembers your choice.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Load models", exact: true }).click();
  await expect(
    page.getByRole("region", { name: "Screen explanation", exact: true }),
  ).toContainText("Two plus two is four");
  await page.evaluate(() =>
    (window as any).fixtureEmit("panel-navigation", {
      id: 11,
      destination: "screenCapture",
    }),
  );
  await expect(
    page.getByText(
      "Answer text sent to the captured app. Check its field; Buddy has not submitted it.",
    ),
  ).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).fixtureCalls
          .filter((c: any) => c.command === "screen_explain")
          .map((c: any) => c.args.fill),
      ),
    )
    .toEqual([false, true]);
  const calls = await page.evaluate(() =>
    (window as any).fixtureCalls.map((c: any) => c.command),
  );
  expect(calls).not.toContain("browser_connections");
  expect(calls).not.toContain("browser_apply");
  await page
    .getByRole("button", { name: "Clear capture", exact: true })
    .click();
  await expect(
    page.getByRole("img", {
      name: "Captured display to review before explaining",
    }),
  ).toHaveCount(0);
});

test("manual screen capture previews locally and recovers from capture failure", async ({
  page,
}) => {
  await page.setViewportSize({ width: 380, height: 800 });
  await installFixture(page, "light");
  await page.goto("/");
  await go(page, "Screen helper");
  await page.evaluate(() => {
    (window as any).fixtureScreenFail = true;
  });
  await page
    .getByRole("button", { name: "Capture screen", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Synthetic capture failed",
  );
  await page.evaluate(() => {
    (window as any).fixtureScreenFail = false;
  });
  await page
    .getByRole("button", { name: "Capture screen", exact: true })
    .click();
  await expect(
    page.getByRole("img", {
      name: "Captured display to review before explaining",
    }),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).fixtureCalls.some(
        (c: any) => c.command === "screen_explain",
      ),
    ),
  ).toBe(false);
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

test("capture scope persists and blocked typing keeps a copyable answer", async ({
  page,
}) => {
  await installFixture(page);
  await page.goto("/");
  await go(page, "Screen helper");
  await page
    .getByRole("combobox", { name: "Capture scope", exact: true })
    .selectOption("area");
  await page.evaluate(() => {
    localStorage.setItem("buddy-screen-model", "fixture-vision");
    (window as any).fixtureTypingError = "Typing blocked. The answer was kept.";
  });
  await page.reload();
  await page
    .getByText("Your focus session finished", { exact: true })
    .waitFor();
  await page.evaluate(() => {
    (window as any).fixtureTypingError = "Typing blocked. The answer was kept.";
  });
  await page.evaluate(() =>
    (window as any).fixtureEmit("panel-navigation", {
      id: 20,
      destination: "screenCapture",
    }),
  );
  await expect(page.getByRole("alert")).toContainText("Typing blocked");
  await expect(
    page.getByRole("heading", { name: "Generated answer", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy answer", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () =>
        (window as any).fixtureCalls.find(
          (c: any) => c.command === "screen_capture",
        ).args.scope,
    ),
  ).toBe("area");
  expect(
    await page.evaluate(
      () =>
        (window as any).fixtureCalls.filter(
          (c: any) => c.command === "screen_explain",
        ).length,
    ),
  ).toBe(1);
});

test("compact explanation displays code and answer with accessible copy actions", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 480 });
  await installFixture(page, "light");
  await page.addInitScript(() => {
    (window as any).fixtureScreenResult = {
      explanation: "Add the two values.\n```python\nprint(2 + 2)\n```",
      answer: "4",
    };
  });
  await page.goto("/?surface=explanation");
  await expect(
    page.getByRole("heading", { name: "Explanation", exact: true }),
  ).toBeVisible();
  await expect(page.locator("pre code")).toContainText("print(2 + 2)");
  await expect(
    page.getByRole("button", { name: "Copy answer", exact: true }),
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

test("loading stays on Kitty and the bubble appears only with a ready answer", async ({page})=>{
  await installFixture(page);
  await page.addInitScript(()=>{(window as any).fixtureScreenResult={title:"Explanation",busy:true};});
  await page.goto("/?surface=explanation");
  await expect(page.locator(".explanation-bubble")).toHaveCount(0);
  expect(await page.evaluate(()=>(window as any).fixtureCalls.filter((c:any)=>c.command==="bubble_present").length)).toBe(0);
  await page.evaluate(()=>{(window as any).fixtureScreenResult={title:"Explanation",explanation:"A short ready answer."};(window as any).fixtureEmit("screen-result",null);});
  await expect(page.getByText("A short ready answer.",{exact:true})).toBeVisible();
  await expect(page.locator(".kitty-thinking")).toHaveCount(0);
});


test.skip("Kitty chats distinguish live tasks, accept explicit follow-ups, and never show toolbar", async ({page}) => {
  await installFixture(page);
  await page.addInitScript(()=>{(window as any).fixtureChats={live:true,data:[{id:"a",name:"Running project",preview:"",updatedAt:1700000000,status:{type:"active"}},{id:"b",name:"Finished project",preview:"",updatedAt:1700000000,status:{type:"idle"}}]};});
  await page.goto("/?surface=chats");
  await expect(page.getByRole("heading",{name:"Codex chats"})).toBeVisible();
  await expect(page.getByRole("navigation")).toHaveCount(0);
  await page.getByRole("button",{name:/Running project/}).click();
  await page.getByLabel("Message to Running project").fill("Please explain the result");
  await page.getByRole("button",{name:"Send to running task"}).click();
  await expect(page.getByRole("status")).toContainText("Message accepted");
  await expect.poll(()=>page.evaluate(()=>(window as any).fixtureCalls.filter((c:any)=>c.command==="kitty_chat_send").length)).toBe(1);
});
test.skip("unloaded chat history never claims idle or enables sending", async ({page})=>{
  await page.setViewportSize({width:320,height:480});await installFixture(page,"light");
  await page.addInitScript(()=>{(window as any).fixtureChats={live:false,data:[{id:"a",name:"Saved project",preview:"",updatedAt:1700000000,status:{type:"notLoaded"}}]};});
  await page.goto("/?surface=chats");await page.getByRole("button",{name:/Saved project/}).click();
  await expect(page.getByText("Status unavailable",{exact:true})).toBeVisible();
  await page.getByLabel("Message to Saved project").fill("Follow up");await expect(page.getByRole("button",{name:"Send message"})).toBeDisabled();
  const results=await new AxeBuilder({page}).withTags(["wcag2a","wcag2aa","wcag21aa","wcag22aa"]).analyze();expect(results.violations).toEqual([]);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)).toBe(false);
});

test("archived entry points stay inaccessible and explanations have no connection chatter",async({page})=>{
  await installFixture(page);await page.goto("/");
  await expect(page.locator(".nav-item").filter({hasText:/^(Codex|Projects)$/})).toHaveCount(0);
  await expect(page.getByText("Continue a project",{exact:true})).toHaveCount(0);
  await page.goto("/?surface=chats");await expect(page.getByRole("heading",{name:"Codex chats",exact:true})).toHaveCount(0);
  await page.addInitScript(()=>{(window as any).fixtureScreenResult={title:"Explanation",question:"What is this?",explanation:"A concise explanation."};});
  await page.goto("/?surface=explanation");await expect(page.getByText("A concise explanation.",{exact:true})).toBeVisible();
  await expect(page.getByText(/Via Codex|account allowance|Asking your AI|What is this\?/)).toHaveCount(0);
  await expect(page.locator("header").getByRole("button",{name:"Copy explanation",exact:true})).toBeVisible();
  await expect(page.getByRole("button",{name:"Copy explanation",exact:true})).toHaveCount(1);
});

test("compact bubble scrolls long answers, sends follow-ups, and dismisses outside clicks",async({page})=>{
  await page.setViewportSize({width:380,height:600});await installFixture(page,"light");
  await page.addInitScript(()=>{(window as any).fixtureScreenResult={title:"Explanation",explanation:"Useful explanation. ".repeat(160)};});
  await page.goto("/?surface=explanation");await page.getByLabel("Follow-up question").waitFor();
  const size=await page.locator(".explanation-bubble").boundingBox();expect(size!.height).toBeLessThanOrEqual(337);
  expect(await page.locator(".bubble-answer").evaluate(e=>e.scrollHeight>e.clientHeight)).toBe(true);
  const scrollbar=await page.locator(".bubble-answer").evaluate(e=>({width:getComputedStyle(e,"::-webkit-scrollbar").width,radius:getComputedStyle(e,"::-webkit-scrollbar-thumb").borderRadius}));
  expect(scrollbar).toEqual({width:"4px",radius:"0px"});
  await page.getByLabel("Follow-up question").fill("Explain with an example");await page.getByRole("button",{name:"Send follow-up",exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>(window as any).fixtureCalls.filter((c:any)=>c.command==="bubble_followup").length)).toBe(1);
  await page.mouse.click(1,1);
  await expect.poll(()=>page.evaluate(()=>(window as any).fixtureCalls.filter((c:any)=>c.command==="plugin:window|close").length)).toBeGreaterThan(0);
  const results=await new AxeBuilder({page}).withTags(["wcag2a","wcag2aa","wcag21aa","wcag22aa"]).analyze();expect(results.violations).toEqual([]);
});
