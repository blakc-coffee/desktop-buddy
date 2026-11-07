import { editorBridge } from "./editors.js";
let port;
let target;
function disconnect() { target = undefined; if (port) { const old = port; port = undefined; old.disconnect(); } }
async function checkTarget() {
  if (!target || Date.now() > target.until) throw Error("Reconnect the page using the Buddy extension.");
  const tab = await chrome.tabs.get(target.tabId);
  if (!tab.active || new URL(tab.url).origin !== target.origin) throw Error("The connected page is no longer active. Reconnect it.");
  return target;
}
async function request(message) {
  const chosen = await checkTarget();
  if (!["capture", "apply"].includes(message.action)) throw Error("Unsupported Buddy request.");
  if (message.action === "capture") {
    await chrome.scripting.executeScript({ target: { tabId: chosen.tabId }, files: ["page.js"] });
  }
  const send = async (type, plan) => {
    const response = await chrome.tabs.sendMessage(chosen.tabId, { type, plan });
    if (!response?.ok) throw Error(response?.error || "The page did not respond. Reconnect it.");
    return response.payload;
  };
  const editor = async (operation, plan) => {
    const results = await chrome.scripting.executeScript({ target: { tabId: chosen.tabId }, world: "MAIN", func: editorBridge, args: [operation, plan || null] });
    if (results[0]?.error) throw Error(results[0].error.message);
    if (!results[0]?.result) throw Error("The editor did not respond.");
    return results[0].result;
  };
  if (message.action === "capture") {
    const page = await send("capture");
    const code = await editor("capture");
    page.fields = [...code.fields, ...page.fields].slice(0, 60);
    page.editorNonce = code.nonce;
    if (!page.fields.length) throw Error("No supported answer controls were found. This page may use an unsupported editor, iframe, or custom control.");
    page.unsupportedEditors = Math.max(0, page.unsupportedEditors - code.fields.length);
    return page;
  }
  const standard = { ...message.plan, assignments: message.plan.assignments.filter((assignment) => !assignment.id.startsWith("editor-")) };
  const code = { ...message.plan, assignments: message.plan.assignments.filter((assignment) => assignment.id.startsWith("editor-")) };
  await send("validate", standard);
  if (code.assignments.length) await editor("validate", code);
  const result = standard.assignments.length ? await send("apply", standard) : { changed: [], error: "" };
  if (result.error || !code.assignments.length) return result;
  try {
    await send("validate", { ...standard, assignments: [] });
    const codeResult = await editor("apply", code);
    return { changed: [...result.changed, ...codeResult.changed], error: codeResult.error };
  } catch (error) { return { changed: result.changed, error: String(error.message || error) }; }

}
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  if (sender.id !== chrome.runtime.id || sender.tab || !sender.url?.startsWith(chrome.runtime.getURL(""))) return;
  if (message.type === "disconnect") { disconnect(); respond({ ok: true }); return; }
  if (message.type !== "connect") return;
  (async () => {
    disconnect();
    const tab = await chrome.tabs.get(message.tabId);
    if (!tab.active || new URL(tab.url).origin !== message.origin) throw Error("The selected page changed.");
    target = { tabId: tab.id, origin: message.origin, title: tab.title?.slice(0, 240) || message.origin, until: Date.now() + 300000 };
    port = chrome.runtime.connectNative("local.dharu.desktopbuddy.page_helper");
    const activePort = port;
    const ready = () => { if (target && port === activePort) activePort.postMessage({ type: "ready", title: target.title, origin: target.origin, until: target.until }); };
    ready();
    activePort.onMessage.addListener((message) => {
      if (message.action === "ping") { ready(); return; }
      void request(message).then((payload) => activePort.postMessage({ type: "response", id: message.id, ok: true, payload }))
        .catch((error) => activePort.postMessage({ type: "response", id: message.id, ok: false, error: String(error.message || error).slice(0, 1000) }));
    });
    activePort.onDisconnect.addListener(() => { if (port === activePort) { port = undefined; target = undefined; } void chrome.runtime.lastError; });
    // Require a successful host handshake rather than reporting a missing host as connected.
    await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(Error("The native helper did not respond. Run Install-Browser-Helper.ps1 first.")), 4000);
      const onMessage = (value) => { if (value.action === "ping") { clearTimeout(timeout); activePort.onMessage.removeListener(onMessage); resolve(); } };
      activePort.onMessage.addListener(onMessage);
    });
    return { ok: true };
  })().then(respond).catch((error) => { disconnect(); respond({ ok: false, error: String(error.message || error) }); });
  return true;
});
chrome.tabs.onRemoved.addListener((id) => { if (target?.tabId === id) disconnect(); });
chrome.tabs.onUpdated.addListener((id, change) => { if (target?.tabId === id && (change.status === "loading" || change.url)) disconnect(); });
