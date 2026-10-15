import { ArrowsClockwise, Code, FolderOpen } from "@phosphor-icons/react";
import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Group, Section } from "../../components/ui/Section";
import { Switch } from "../../components/ui/Switch";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import { Mark, PageTitle } from "../App";
import { useData } from "../data";

const t = strings.about;

export function AboutPage() {
  const { info, settings, update } = useData();
  return (
    <>
      <PageTitle>{t.title}</PageTitle>
      <div className="mb-9 flex items-center gap-3.5">
        <Mark size={40} />
        <div>
          <p className="font-semibold">{strings.app.name}</p>
          <p className="font-mono text-[12px] text-muted">{t.version(info.version)}</p>
        </div>
      </div>
      <Section title={t.updates}>
        <Group>
          <Switch label={t.autoUpdate} hint={t.autoUpdateHint} checked={settings.check_updates} onChange={(check_updates) => void update({ check_updates })} />
          <CheckNow enabled={info.updates} />
        </Group>
      </Section>
      <Section title={t.privacyTitle}>
        <p className="max-w-[60ch] leading-relaxed text-muted">{t.privacy}</p>
      </Section>
      <Section title={t.credits}>
        <p className="max-w-[60ch] leading-relaxed text-muted">{t.creditsList}</p>
      </Section>
      <Section title={t.help}>
        <Group className="flex flex-wrap gap-2 p-3">
          <Button size="sm" onClick={() => void api.openLogs()}>
            <FolderOpen size={14} /> {t.logs}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void api.openUrl("https://github.com/omikhan4901/text_explainer")}>
            <Code size={14} /> {t.source}
          </Button>
        </Group>
      </Section>
    </>
  );
}

/** "Check now", and what it found. A new version is offered by the app's own dialog. */
function CheckNow({ enabled }: { enabled: boolean }) {
  const [status, setStatus] = useState<string | null>(enabled ? null : t.unavailable);
  const [checking, setChecking] = useState(false);
  const check = async () => {
    setChecking(true);
    try {
      const r = await api.checkUpdates();
      setStatus(r.state === "up_to_date" ? t.upToDate : r.state === "declined" ? t.declined(r.version) : r.state === "busy" ? t.busy : t.unavailable);
    } catch {
      setStatus(t.failed);
    } finally {
      setChecking(false);
    }
  };
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <Button size="sm" disabled={!enabled || checking} onClick={() => void check()}>
        <ArrowsClockwise size={14} /> {checking ? t.checking : t.checkNow}
      </Button>
      <p role="status" className="text-[13px] text-muted">
        {status}
      </p>
    </div>
  );
}
