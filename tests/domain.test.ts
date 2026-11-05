import { test } from "node:test";
import assert from "node:assert/strict";
import {
  parseCapture,
  validateBackup,
  emptyItem,
  formatDuration,
} from "../src/domain.ts";
test("explicit timer commands enforce duration bounds", () => {
  assert.deepEqual(parseCapture("timer 25m review tests"), {
    type: "timer",
    minutes: 25,
    title: "review tests",
  });
  assert.equal(parseCapture("timer 0m").type, "error");
  assert.equal(parseCapture("timer 1441m").type, "error");
  assert.equal(parseCapture("timer later").type, "error");
});
test("reminder parsing uses local calendar dates and rejects past/ambiguous times", () => {
  const now = new Date(2026, 10, 5, 15, 0);
  const p = parseCapture("remind tomorrow 10:00 review PR", now);
  assert.equal(p.type, "reminder");
  if (p.type === "reminder")
    assert.equal(
      new Date(p.dueAt).getTime(),
      new Date(2026, 10, 6, 10, 0).getTime(),
    );
  assert.equal(parseCapture("remind today 10:00 review PR", now).type, "error");
  assert.equal(
    parseCapture("remind tomorrow 25:00 review PR", now).type,
    "error",
  );
  assert.equal(parseCapture("remind tomorrow 10 review PR", now).type, "error");
});
test("plain text is only a local todo, never an external instruction", () => {
  assert.deepEqual(parseCapture("run a shell command"), {
    type: "todo",
    title: "run a shell command",
  });
  assert.equal(parseCapture("   ").type, "error");
});
test("backup fails closed for unsupported schemas and invalid item payloads", () => {
  assert.throws(() => validateBackup('{"version":9,"items":[]}'));
  assert.throws(() =>
    validateBackup(
      JSON.stringify({
        version: 1,
        items: [{ id: "a", kind: "shell", title: "x" }],
      }),
    ),
  );
  assert.equal(
    validateBackup(
      JSON.stringify({
        version: 1,
        items: [{ ...emptyItem("todo", "hello"), id: "a" }],
      }),
    ).length,
    1,
  );
});
test("duration does not display negative or premature zero seconds", () => {
  assert.equal(formatDuration(-1), "00:00");
  assert.equal(formatDuration(1), "00:01");
  assert.equal(formatDuration(25 * 60000), "25:00");
});
test("a backup at the supported 2000-item ceiling remains importable", () => {
  const items = Array.from({ length: 2000 }, (_, n) => ({
    ...emptyItem("todo", "entry"),
    id: String(n),
  }));
  assert.equal(
    validateBackup(JSON.stringify({ version: 1, items })).length,
    2000,
  );
  assert.throws(() =>
    validateBackup(
      JSON.stringify({
        version: 1,
        items: [...items, { ...items[0], id: "overflow" }],
      }),
    ),
  );
});
