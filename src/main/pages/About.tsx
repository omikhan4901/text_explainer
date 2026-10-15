import { Code, FolderOpen } from "@phosphor-icons/react";
import { Button } from "../../components/ui/Button";
import { Group, Section } from "../../components/ui/Section";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import { Mark, PageTitle } from "../App";
import { useData } from "../data";

const t = strings.about;

export function AboutPage() {
  const { info } = useData();
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
