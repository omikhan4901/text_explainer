import { strings } from "../i18n";

export function Popup() {
  return (
    <div className="m-2 rounded-[var(--radius-card)] border border-border bg-surface p-4 shadow-[var(--shadow-card)]">
      <p className="text-sm text-muted">{strings.app.name}</p>
    </div>
  );
}
