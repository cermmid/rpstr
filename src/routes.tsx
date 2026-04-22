import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./App";
import { UnlockView } from "./features/auth/UnlockView";
import { VisitList } from "./features/visit/VisitList";
import { VisitRecorder } from "./features/visit/VisitRecorder";
import { PrintView } from "./features/visit/PrintView";
import { SettingsView } from "./features/settings/SettingsView";
import { ModelSetupWizard } from "./features/setup/ModelSetupWizard";

export const router = createBrowserRouter([
  { path: "/", element: <Navigate to="/unlock" replace /> },
  { path: "/unlock", element: <UnlockView /> },
  { path: "/setup", element: <ModelSetupWizard /> },
  {
    path: "/app",
    element: <AppShell />,
    children: [
      { index: true, element: <Navigate to="visits" replace /> },
      { path: "visits", element: <VisitList /> },
      { path: "visits/new", element: <VisitRecorder /> },
      { path: "visits/:id", element: <VisitRecorder /> },
      { path: "visits/:id/print", element: <PrintView /> },
      { path: "settings", element: <SettingsView /> },
    ],
  },
]);
