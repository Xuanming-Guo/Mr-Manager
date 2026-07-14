import { Eye, LockKeyhole, ShieldCheck } from "lucide-react";
import { useCapabilitiesQuery } from "../app/queries";
import { AvailabilityBadge } from "../components/AvailabilityBadge";
import { ErrorState, LoadingState } from "../components/AsyncState";

export function CapabilitiesPage() {
  const report = useCapabilitiesQuery();
  if (report.isLoading) return <LoadingState label="Checking Windows capabilities…" />;
  if (report.error || !report.data)
    return <ErrorState error={report.error} retry={() => void report.refetch()} />;

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Transparency center</span>
          <h1>Data access & evidence</h1>
        </div>
        <div className="standard-user-badge">
          <ShieldCheck size={16} />
          {report.data.standardUserMode ? "Standard-user mode" : "Privilege state unknown"}
        </div>
      </header>
      <section className="permission-summary">
        <LockKeyhole size={24} />
        <div>
          <strong>Read-only inspection by default</strong>
          <span>
            System collectors do not change processes, ports, firewall rules, routes, VPN settings,
            files, or Docker state.
          </span>
        </div>
      </section>
      <section className="evidence-explainer-grid">
        <article>
          <strong>Certain</strong>
          <span>Direct operating-system or supervisor evidence, such as a PID owning a port.</span>
        </article>
        <article>
          <strong>Strong</strong>
          <span>Multiple reliable signals, such as a process working inside a project root.</span>
        </article>
        <article>
          <strong>Inferred</strong>
          <span>A useful clue that may be wrong, such as guessing HTTP from a common port.</span>
        </article>
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Platform: {report.data.platform}</span>
            <h2>Feature availability</h2>
          </div>
          <Eye size={18} />
        </div>
        <div className="capability-grid">
          {report.data.features.map((feature) => (
            <article className="capability-card" key={feature.id}>
              <div>
                <strong>{feature.label}</strong>
                {feature.readOnly && <span className="read-only-pill">Read only</span>}
              </div>
              <AvailabilityBadge state={feature.availability.state} />
              {feature.availability.reason && <p>{feature.availability.reason}</p>}
              {feature.availability.remediation && (
                <small>{feature.availability.remediation}</small>
              )}
            </article>
          ))}
        </div>
      </section>
      <section className="security-note">
        <h2>Current security boundary</h2>
        <ul>
          <li>Mr Manager does not request administrator access for normal monitoring.</li>
          <li>The main webview receives only typed, allowlisted commands.</li>
          <li>No generic shell or broad filesystem capability is exposed.</li>
          <li>External network checks are off by default.</li>
          <li>Expected access-denied states are reported without elevation.</li>
        </ul>
      </section>
    </div>
  );
}
