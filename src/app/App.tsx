import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./AppShell";
import { CapabilitiesPage } from "../routes/CapabilitiesPage";
import { CleanerPage } from "../routes/CleanerPage";
import { DockerPage } from "../routes/DockerPage";
import { IntegrationsPage } from "../routes/IntegrationsPage";
import { NetworkPage } from "../routes/NetworkPage";
import { OverviewPage } from "../routes/OverviewPage";
import { ProcessesPage } from "../routes/ProcessesPage";
import { ProjectsPage } from "../routes/ProjectsPage";
import { SettingsPage } from "../routes/SettingsPage";
import { TopologyPage } from "../routes/TopologyPage";
import { SystemDiagnosticsPage } from "../routes/SystemDiagnosticsPage";

export function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<OverviewPage />} />
        <Route path="processes" element={<ProcessesPage />} />
        <Route path="projects" element={<ProjectsPage />} />
        <Route path="topology" element={<TopologyPage />} />
        <Route path="docker" element={<DockerPage />} />
        <Route path="integrations" element={<IntegrationsPage />} />
        <Route path="network" element={<NetworkPage />} />
        <Route path="system-diagnostics" element={<SystemDiagnosticsPage />} />
        <Route path="war-room" element={<Navigate to="/system-diagnostics" replace />} />
        <Route path="cleaner" element={<CleanerPage />} />
        <Route path="portfolio" element={<Navigate to="/projects" replace />} />
        <Route path="permissions" element={<CapabilitiesPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
