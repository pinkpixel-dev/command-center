import React from "react";
import ReactDOM from "react-dom/client";

import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";

import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/layout.css";
import "./styles/components.css";
import "./styles/cards.css";
import "./styles/overlays.css";
import "./styles/import.css";
import "./styles/explanation.css";
import "./styles/assistant.css";
import "./styles/diagnosis.css";
import "./styles/bulk.css";
import "./styles/signin.css";
import "./styles/polish.css";

import App from "./App";
import { AuthGate } from "./components/AuthGate";
import { ToastProvider } from "./components/ui/Toast";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ToastProvider>
      {/* On the desktop this renders App and nothing else. On the server it
          holds the library back until there is a session. */}
      <AuthGate>
        <App />
      </AuthGate>
    </ToastProvider>
  </React.StrictMode>,
);
