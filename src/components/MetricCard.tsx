import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

interface MetricCardProps {
  label: string;
  value: string;
  detail: string;
  icon: LucideIcon;
  percent?: number;
  accent?: "cyan" | "violet" | "amber" | "green";
  footer?: ReactNode;
}

export function MetricCard({
  label,
  value,
  detail,
  icon: Icon,
  percent,
  accent = "cyan",
  footer,
}: MetricCardProps) {
  const clamped = Math.max(0, Math.min(100, percent ?? 0));
  return (
    <article className={`metric-card metric-${accent}`}>
      <div className="metric-heading">
        <span>{label}</span>
        <Icon size={17} aria-hidden="true" />
      </div>
      <div className="metric-value">{value}</div>
      <div className="metric-detail">{detail}</div>
      {percent !== undefined && (
        <div className="metric-track" aria-hidden="true">
          <span style={{ width: `${clamped}%` }} />
        </div>
      )}
      {footer && <div className="metric-footer">{footer}</div>}
    </article>
  );
}
