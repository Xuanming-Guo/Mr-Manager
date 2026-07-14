import { AlertTriangle, LoaderCircle, LockKeyhole } from "lucide-react";
import { normalizeAppError } from "../lib/ipc";

export function LoadingState({ label = "Collecting real system data..." }: { label?: string }) {
  return (
    <div className="center-state" role="status">
      <LoaderCircle className="spin" size={24} aria-hidden="true" />
      <span>{label}</span>
    </div>
  );
}

export function ErrorState({ error, retry }: { error: unknown; retry?: () => void }) {
  const details = normalizeAppError(error);
  const Icon = details.permissionRelevant ? LockKeyhole : AlertTriangle;
  return (
    <section className="error-panel" role="alert">
      <Icon size={22} aria-hidden="true" />
      <div>
        <strong>{details.message}</strong>
        <span>{details.remediation}</span>
        <code>{details.code}</code>
      </div>
      {retry && details.retryable && (
        <button type="button" className="button button-secondary" onClick={retry}>
          Retry
        </button>
      )}
    </section>
  );
}
