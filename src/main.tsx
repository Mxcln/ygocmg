import React from "react";
import ReactDOM from "react-dom/client";
import { AppProviders } from "./app/providers";
import { App } from "./app/App";
import { bootstrapThemeFromMirror } from "./shared/theme/theme";
import "./app/styles.css";

bootstrapThemeFromMirror();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppProviders>
      <App />
    </AppProviders>
  </React.StrictMode>,
);
