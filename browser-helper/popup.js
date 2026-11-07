const status = document.querySelector("#status");
const connect = document.querySelector("#connect");
let currentTab;
connect.disabled = true;
void chrome.tabs.query({ active: true, currentWindow: true }).then(([tab]) => {
  if (!tab?.url || !["http:", "https:"].includes(new URL(tab.url).protocol)) { status.textContent = "Open an ordinary website first."; return; }
  currentTab = tab;
  connect.disabled = false;
}).catch((error) => { status.textContent = String(error.message || error); });
document.querySelector("#extension-id").textContent = chrome.runtime.id;
connect.addEventListener("click", async () => {
  try {
    const tab = currentTab;
    if (!tab) throw Error("The page is not ready to connect.");
    const url = new URL(tab.url);
    if (!["http:", "https:"].includes(url.protocol)) throw Error("Open an ordinary website first.");
    const granted = await chrome.permissions.request({ origins: [`${url.origin}/*`] });
    if (!granted) throw Error("Page access was not granted.");
    const result = await chrome.runtime.sendMessage({ type: "connect", tabId: tab.id, origin: url.origin });
    if (!result.ok) throw Error(result.error);
    status.textContent = "Connected for five minutes. Use /fill in Buddy.";
  } catch (error) { status.textContent = String(error.message || error); }
});
document.querySelector("#disconnect").addEventListener("click", async () => { await chrome.runtime.sendMessage({ type: "disconnect" }); status.textContent = "Disconnected."; });
