import { chromium } from "playwright";
import AxeBuilder from "@axe-core/playwright";
import { mkdir, writeFile } from "node:fs/promises";
import { installFixture } from "./fixture";
const output = process.env.BUDDY_AUDIT_OUTPUT || "artifacts/ui-audit";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const findings: unknown[] = [];
try {
  for (const theme of ["dark", "light"])
    for (const width of [1100, 380]) {
      const context = await browser.newContext({
        viewport: { width, height: 800 },
      });
      const page = await context.newPage();
      await installFixture(page, theme);
      await page.goto(process.env.BUDDY_AUDIT_URL || "http://127.0.0.1:1437/");
      await page.waitForFunction(
        () =>
          document.querySelector(".runtime-label")?.textContent?.trim() ===
          "DESKTOP",
      );
      await page
        .getByText("Your focus session finished", { exact: true })
        .waitFor();
      await page.locator(".loading").waitFor({ state: "hidden" });
      for (const name of [
        "Overview",
        "Tasks",
        "Projects",
        "Shelf",
        "Codex",
        "Browser helper",
        "Music",
        "Settings",
      ]) {
        const toggle = page.getByRole("button", { name: /^Tools/ });
        if (
          (await toggle.isVisible()) &&
          (await toggle.getAttribute("aria-expanded")) === "false"
        )
          await toggle.click();
        await page
          .locator(".nav-item")
          .filter({ hasText: new RegExp(`^${name}`) })
          .click();
        await page.evaluate(() => scrollTo(0, 0));
        await page.screenshot({
          path: `${output}/${name}-${theme}-${width}.png`,
          fullPage: true,
        });
        const result = await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
          .analyze();
        findings.push({
          name,
          theme,
          width,
          overflow: await page.evaluate(
            () => document.documentElement.scrollWidth > innerWidth,
          ),
          violations: result.violations.map(({ id, impact, nodes }) => ({
            id,
            impact,
            nodes: nodes.map(({ target, failureSummary }) => ({
              target,
              failureSummary,
            })),
          })),
        });
      }
      await context.close();
    }
} finally {
  await browser.close();
}
await writeFile(`${output}/findings.json`, JSON.stringify(findings, null, 2));
console.log(
  JSON.stringify(
    findings.map((value: any) => ({
      name: value.name,
      width: value.width,
      overflow: value.overflow,
      violations: value.violations.map((item: any) => item.id),
    })),
    null,
    2,
  ),
);
