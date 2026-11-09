import React from "react";
import { createRoot } from "react-dom/client";
import View from "./Todo";
import "./style.css";
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <View />
  </React.StrictMode>,
);
