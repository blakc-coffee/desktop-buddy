import { test } from "node:test";
import assert from "node:assert/strict";
import { destinationPage } from "../src/panelNavigation.ts";
test("native destinations open the matching management surface", () => {
  assert.equal(destinationPage("screenCapture"), "Fill");
  assert.equal(destinationPage("screenExplain"), "Fill");
  assert.equal(destinationPage("ask"), "Settings");
  assert.equal(destinationPage("resume"), "Settings");
  assert.equal(destinationPage("settings"), "Settings");
  assert.equal(destinationPage("projects"), "Settings");
  assert.equal(destinationPage("shelf"), "Shelf");
  assert.equal(destinationPage("timers"), "Tasks");
  assert.equal(destinationPage("reminders"), "Tasks");
});

import { pageEnabled } from "../src/features.ts";
test("archived pages cannot be selected while everyday tools remain available",()=>{
  assert.equal(pageEnabled("Codex"),false);assert.equal(pageEnabled("Projects"),false);
  for(const page of ["Tasks","Fill","Settings","Activity"]) assert.equal(pageEnabled(page),true);
});
