import * as Tabs from "@radix-ui/react-tabs";
import { Activity, RefreshCw, Radio } from "lucide-react";
import { usePortsQuery, useProcessesQuery, useSettingsQuery } from "../app/queries";
import { useQuery } from "@tanstack/react-query";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { PortTable } from "../components/PortTable";
import { ProcessTable } from "../components/ProcessTable";
import { projectsApi } from "../lib/projects-ipc";

export function ProcessesPage() {
  const settings = useSettingsQuery();
  const processes = useProcessesQuery(settings.data?.refreshMode);
  const ports = usePortsQuery(settings.data?.refreshMode);
  const projects = useQuery({ queryKey: ["projects"], queryFn: projectsApi.list });
  const refresh = () => {
    void processes.refetch();
    void ports.refetch();
  };

  return (
    <div className="page-stack page-fill">
      <header className="page-header">
        <div>
          <span className="eyebrow">Ownership-aware inspection</span>
          <h1>Processes & ports</h1>
        </div>
        <button className="button button-secondary" type="button" onClick={refresh}>
          <RefreshCw size={15} className={processes.isFetching || ports.isFetching ? "spin" : ""} />
          Refresh
        </button>
      </header>
      <Tabs.Root defaultValue="processes" className="tabs-root">
        <Tabs.List className="tabs-list" aria-label="Inspector view">
          <Tabs.Trigger value="processes">
            <Activity size={15} />
            Processes <span>{processes.data?.length ?? "—"}</span>
          </Tabs.Trigger>
          <Tabs.Trigger value="ports">
            <Radio size={15} />
            Listening ports <span>{ports.data?.length ?? "—"}</span>
          </Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="processes" className="tabs-content">
          {processes.isLoading ? (
            <LoadingState label="Enumerating processes…" />
          ) : processes.error || !processes.data ? (
            <ErrorState error={processes.error} retry={() => void processes.refetch()} />
          ) : (
            <ProcessTable processes={processes.data} projects={projects.data ?? []} />
          )}
        </Tabs.Content>
        <Tabs.Content value="ports" className="tabs-content">
          {ports.isLoading ? (
            <LoadingState label="Mapping port ownership…" />
          ) : ports.error || !ports.data ? (
            <ErrorState error={ports.error} retry={() => void ports.refetch()} />
          ) : (
            <PortTable endpoints={ports.data} />
          )}
        </Tabs.Content>
      </Tabs.Root>
    </div>
  );
}
