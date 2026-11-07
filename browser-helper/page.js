(() => {
  if (globalThis.__buddyPageHelper) return;
  globalThis.__buddyPageHelper = true;
  let captured;
  const receipts = new Map();
  const visible = (element) => element.isConnected && !element.closest('[hidden],[aria-hidden="true"]') && element.getClientRects().length > 0 && getComputedStyle(element).visibility !== "hidden";
  const question = () => (document.querySelector("main,article,[role=main],form") || document.body).innerText.slice(0, 20000);
  const hash = async (text) => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)))].map((value) => value.toString(16).padStart(2, "0")).join("");
  function label(element) {
    return (element.labels?.[0]?.innerText || element.getAttribute("aria-label") || (element.getAttribute("aria-labelledby") || "").split(/\s+/).map((id) => document.getElementById(id)?.innerText || "").join(" ") || element.closest("fieldset")?.querySelector("legend")?.innerText || element.closest("label")?.innerText || element.getAttribute("placeholder") || element.name || "Answer").trim().slice(0, 500);
  }
  function read(field) {
    if (field.kind === "radio") return String(field.elements.findIndex((element) => element.checked || element.getAttribute("aria-checked") === "true"));
    const element = field.elements[0];
    if (field.kind === "checkbox") return String(element.checked ?? (element.getAttribute("aria-checked") === "true"));
    return element.value;
  }
  function fields() {
    const result = [];
    const seen = new Set();
    const controls = [...document.querySelectorAll('textarea,input:not([type]),input[type=text],input[type=number],input[type=radio],input[type=checkbox],select,[role=radio],[role=checkbox]')].filter((element) => visible(element) && !element.disabled && !element.readOnly && element.getAttribute("aria-disabled") !== "true");
    for (const element of controls) {
      if (seen.has(element) || result.length >= 60) continue;
      if (element.closest('.monaco-editor,.CodeMirror,.cm-editor,[role=search]') || element.autocomplete && /password|email|tel|cc-|address|name|bday/i.test(element.autocomplete)) continue;
      const kind = element.type === "radio" || element.getAttribute("role") === "radio" ? "radio" : element.type === "checkbox" || element.getAttribute("role") === "checkbox" ? "checkbox" : element.tagName === "SELECT" ? "select" : "text";
      if (element.multiple) continue;
      let elements = [element];
      if (kind === "radio") {
        const group = element.closest('[role=radiogroup]');
        elements = group ? controls.filter((candidate) => candidate.closest('[role=radiogroup]') === group && candidate.getAttribute("role") === "radio") : element.name ? controls.filter((candidate) => candidate.type === "radio" && candidate.name === element.name && candidate.form === element.form) : [element];
      }
      elements.forEach((item) => seen.add(item));
      const field = { id: `field-${result.length + 1}`, kind, label: label(kind === "radio" ? element.closest("fieldset,[role=radiogroup]") || element : element), elements };
      field.options = kind === "radio" ? elements.map((item, index) => ({ value: String(index), label: label(item) })) : kind === "select" ? [...element.options].filter((option) => !option.disabled).map((option) => ({ value: option.value, label: option.textContent.trim().slice(0, 300) })) : [];
      if (kind === "radio" && field.options.length > 30 || field.options.length > 100) continue;
      field.value = read(field);
      if (field.value.length > 20000) continue;
      result.push(field);
    }
    return result;
  }
  async function capture() {
    const found = fields();

    const text = question();
    captured = { nonce: crypto.randomUUID(), fingerprint: await hash(text), question: text, fields: found };
    return { nonce: captured.nonce, fingerprint: captured.fingerprint, title: document.title.slice(0, 240), origin: location.origin, question: text, fields: found.map(({ elements, ...field }) => field), unsupportedEditors: document.querySelectorAll('.monaco-editor,.CodeMirror,.cm-editor').length };
  }
  function setText(element, value) {
    const prototype = element.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : element.tagName === "SELECT" ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype, "value").set.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
  }
  async function apply(plan, validateOnly = false) {
    if (!validateOnly && receipts.has(plan?.id)) return receipts.get(plan.id);
    if (!captured || plan.nonce !== captured.nonce || plan.fingerprint !== captured.fingerprint || await hash(question()) !== captured.fingerprint) throw Error("The page changed. Capture it again before filling.");
    if (captured.used && !validateOnly) throw Error("This capture was already applied. Capture the page again.");
    if (typeof plan.id !== "string" || !Array.isArray(plan.assignments) || (!validateOnly && !plan.assignments.length) || plan.assignments.length > 60) throw Error("Invalid answer plan.");
    const ids = new Set();
    const actions = plan.assignments.map((assignment) => {
      const field = captured.fields.find((field) => field.id === assignment.id);
      if (!field || ids.has(field.id) || typeof assignment.value !== "string" || assignment.value.length > 20000) throw Error("The answer plan contains an invalid or repeated field.");
      ids.add(field.id);
      if (field.elements.some((element) => !visible(element) || element.disabled || element.readOnly) || read(field) !== field.value) throw Error("An answer control changed or was edited. Capture the page again.");
      if (field.kind === "checkbox" && !["true", "false"].includes(assignment.value)) throw Error("Invalid checkbox answer.");
      if (["select", "radio"].includes(field.kind) && !field.options.some((option) => option.value === assignment.value)) throw Error("The answer is not one of the available options.");
      return { field, value: assignment.value };
    });
    if (validateOnly) return { changed: [], error: "" };
    const result = { changed: [], error: "" };
    for (const { field, value } of actions) {
      try {
        if (field.elements.some((element) => !visible(element)) || read(field) !== field.value) throw Error("The page changed while filling. Remaining answers were left alone.");
        if (field.kind === "radio") {
          if (read(field) !== value) field.elements[Number(value)].click();
        } else if (field.kind === "checkbox") {
          if (read(field) !== value) field.elements[0].click();
        } else setText(field.elements[0], value);
        await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        if (read(field) !== value) throw Error("The page did not retain this answer. Remaining answers were left alone.");
        result.changed.push(field.id);
      } catch (error) { result.error = String(error.message || error); break; }
    }
    receipts.set(plan.id, result);
    if (receipts.size > 5) receipts.delete(receipts.keys().next().value);
    captured.used = true;
    return result;
  }
  chrome.runtime.onMessage.addListener((message, sender, respond) => {
    if (sender.id !== chrome.runtime.id || !["capture", "validate", "apply"].includes(message.type)) return;
    (message.type === "capture" ? capture() : apply(message.plan, message.type === "validate")).then((payload) => respond({ ok: true, payload })).catch((error) => respond({ ok: false, error: String(error.message || error) }));
    return true;
  });
})();
