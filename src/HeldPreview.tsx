import { useEffect, useState } from "react";
import { call, native } from "./bridge";
import type { Snapshot } from "./domain";
type Preview = {
  name: string;
  kind: "image" | "text" | "link" | "details";
  text?: string;
  image?: string;
  message?: string;
  error?: string;
  truncated?: boolean;
  size?: number;
};
export default function HeldPreview() {
  const [value, setValue] = useState<Preview | null>(null),
    [error, setError] = useState(""),
    [copied, setCopied] = useState(false);
  useEffect(() => {
    let alive = true,
      off = () => {};
    const refresh = () =>
      void call<Preview | null>("held_preview_read")
        .then((value) => {
          if (alive) {
            setValue(value);
            setCopied(false);
            setError("");
          }
        })
        .catch((error) => {
          if (alive) setError(String(error));
        });
    void call<Snapshot>("snapshot")
      .then((snapshot) => {
        if (alive) {
          document.documentElement.dataset.theme = snapshot.theme;
          document.documentElement.dataset.pet =
            snapshot.petAppearance || "calico";
        }
      })
      .catch(() => {});
    if (native)
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const stop = await listen("held-preview-changed", refresh);
        if (alive) {
          off = stop;
          refresh();
        } else stop();
      });
    else refresh();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape" && native)
        void import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
          getCurrentWindow().close(),
        );
    };
    window.addEventListener("keydown", key);
    return () => {
      alive = false;
      off();
      window.removeEventListener("keydown", key);
    };
  }, []);
  return (
    <main className="held-preview">
      <header>
        <h1>{value?.name || "Preview"}</h1>
        {value?.text && (
          <button
            className="small-button"
            onClick={() =>
              void navigator.clipboard
                .writeText(value.text!)
                .then(() => setCopied(true))
                .catch((error) => setError(String(error)))
            }
          >
            Copy
          </button>
        )}
      </header>
      {error && <p role="alert">{error}</p>}
      {value?.error && <p role="alert">{value.error}</p>}
      <div
        className="held-preview-content"
        tabIndex={0}
        role="region"
        aria-label="File preview"
      >
        {value?.kind === "image" && <img src={value.image} alt={value.name} />}
        {value?.text && <pre>{value.text}</pre>}
        {value?.message && <p>{value.message}</p>}
        {value?.truncated && <p>Showing the first 64 KiB.</p>}
      </div>
      <footer>
        {value && (
          <button
            className="small-button"
            onClick={() =>
              void call("held_preview_open").catch((error) =>
                setError(String(error)),
              )
            }
          >
            {value.kind === "link" ? "Open link" : "Open in app"}
          </button>
        )}
        <span role="status">{copied ? "Copied" : ""}</span>
      </footer>
    </main>
  );
}
