// Runs in the page's main world. Only these authored editor API calls are allowed;
// no generated selectors, JavaScript, submission, execution, or language changes.
export function editorBridge(operation, plan) {
  const visible = (element) => element.isConnected && element.getClientRects().length > 0 && getComputedStyle(element).visibility !== "hidden";
  if (operation === "capture") {
    const records = [];
    const monacoElements = [...document.querySelectorAll(".monaco-editor")].filter(visible);
    const models = window.monaco?.editor?.getModels?.() || [];
    if (monacoElements.length === 1 && models.length === 1 && !models[0].isDisposed()) records.push({ element: monacoElements[0], api: models[0], adapter: "Monaco", language: () => models[0].getLanguageId(), read: () => models[0].getValue(), write: (value) => models[0].setValue(value) });
    for (const element of [...document.querySelectorAll(".CodeMirror")].filter(visible)) {
      const api = element.CodeMirror;
      if (api?.getValue && api?.setValue && !api.getOption("readOnly")) records.push({ element, api, adapter: "CodeMirror 5", document: api.getDoc(), language: () => String(api.getOption("mode")?.name || api.getOption("mode") || "plain text"), read: () => api.getValue(), write: (value) => api.setValue(value) });
    }
    for (const element of [...document.querySelectorAll(".ace_editor")].filter(visible)) {
      const api = element.env?.editor;
      if (api?.getValue && api?.setValue && !api.getReadOnly()) records.push({ element, api, adapter: "Ace", document: api.getSession(), language: () => String(api.getSession().getMode().$id || "plain text"), read: () => api.getValue(), write: (value) => api.setValue(value, -1) });
    }
    const nonce = crypto.randomUUID();
    const fields = [];
    const selected = [];
    for (const record of records.slice(0, 8)) {
      const value = record.read();
      if (typeof value !== "string" || value.length > 20000) continue;
      record.id = `editor-${fields.length + 1}`; record.value = value; record.originalLanguage = record.language().slice(0, 100);
      fields.push({ id: record.id, kind: "text", label: `Code answer (${record.adapter}, ${record.originalLanguage}; keep this language)`, value, options: [], adapter: record.adapter });
      selected.push(record);
    }
    window.__buddyEditorState = { nonce, records: selected, receipts: new Map(), created: Date.now() };
    return { nonce, fields };
  }
  const state = window.__buddyEditorState;
  if (!state || state.nonce !== plan.editorNonce || Date.now() - state.created > 300000) throw Error("The editor capture expired. Capture again.");
  if (state.receipts.has(plan.id)) return state.receipts.get(plan.id);
  const ids = new Set();
  const actions = plan.assignments.map((assignment) => {
    const record = state.records.find((record) => record.id === assignment.id);
    if (!record || ids.has(record.id) || typeof assignment.value !== "string" || assignment.value.length > 20000) throw Error("Invalid code answer plan.");
    ids.add(record.id);
    if (!visible(record.element) || record.element.querySelector('[aria-readonly="true"]') || record.read() !== record.value || record.api.isDisposed?.()) throw Error("The code editor changed. Capture again.");
    if (record.adapter === "CodeMirror 5" && (record.element.CodeMirror !== record.api || record.api.getDoc() !== record.document || record.api.getOption("readOnly"))) throw Error("The code editor changed or became read-only.");
    if (record.adapter === "Ace" && (record.element.env?.editor !== record.api || record.api.getSession() !== record.document || record.api.getReadOnly())) throw Error("The code editor changed or became read-only.");
    if (record.adapter === "Monaco" && (window.monaco?.editor?.getModels?.().length !== 1 || window.monaco.editor.getModels()[0] !== record.api)) throw Error("The active code model changed.");
    if (record.language().slice(0, 100) !== record.originalLanguage) throw Error("The coding language changed. Capture again.");
    return { record, value: assignment.value };
  });
  if (operation === "validate") return { changed: [], error: "" };
  if (operation !== "apply") throw Error("Unsupported editor operation.");
  const result = { changed: [], error: "" };
  for (const { record, value } of actions) {
    try {
      if (!visible(record.element) || record.read() !== record.value) throw Error("The editor changed while filling. Remaining answers were left alone.");
      record.write(value);
      if (record.read() !== value) throw Error("The editor did not retain this code answer.");
      result.changed.push(record.id);
    } catch (error) { result.error = String(error.message || error); break; }
  }
  state.receipts.set(plan.id, result);
  return result;
}
