import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import Todo from "./Todo";
import Explanation from "./Explanation";
import "./style.css";
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {(window as unknown as { __BUDDY_SURFACE__?: string }).__BUDDY_SURFACE__ ===
      "todo" ||
    new URLSearchParams(location.search).get("surface") === "todo" ? (
      <Todo />
    ) : new URLSearchParams(location.search).get("surface") ===
      "explanation" ? (
      <Explanation />
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
