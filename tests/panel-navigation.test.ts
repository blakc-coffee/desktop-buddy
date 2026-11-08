import { test } from "node:test";
import assert from "node:assert/strict";
import { destinationPage } from "../src/panelNavigation.ts";
test("native destinations open the matching management surface", () => {
  assert.equal(destinationPage("screenCapture"), "Fill");
  assert.equal(destinationPage("screenExplain"), "Fill");
  assert.equal(destinationPage("ask"), "Codex");
  assert.equal(destinationPage("resume"), "Projects");
  assert.equal(destinationPage("settings"), "Settings");
  assert.equal(destinationPage("projects"), "Projects");
  assert.equal(destinationPage("shelf"), "Shelf");
  assert.equal(destinationPage("timers"), "Tasks");
  assert.equal(destinationPage("reminders"), "Tasks");
});
