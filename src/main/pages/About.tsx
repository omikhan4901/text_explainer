import { Code2, FolderOpen } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Card, CardTitle } from "../../components/ui/Card";
import { strings } from "../../i18n";
import { api } from "../../lib/ipc";
import { PageTitle } from "../App";
import { useData } from "../data";

const t = strings.about;

export function AboutPage() {
  const { info } = useData();
  return (
    <>
      <PageTitle lead={t.version(info.version)}>{t.title}</PageTitle>
      <Card>
        <CardTitle>{t.privacyTitle}</CardTitle>
        <p className="leading-relaxed text-muted">{t.privacy}</p>
      </Card>
      <Card className="mt-4">
        <CardTitle>{t.credits}</CardTitle>
        <p className="leading-relaxed text-muted">{t.creditsList}</p>
        <div className="mt-4 flex flex-wrap gap-2">
          <Button size="sm" onClick={() => void api.openLogs()}>
            <FolderOpen size={15} /> {t.logs}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void api.openUrl("https://github.com/omikhan4901/text_explainer")}>
            <Code2 size={15} /> {t.source}
          </Button>
        </div>
      </Card>
    </>
  );
}
