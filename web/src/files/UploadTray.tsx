// The uploads on their way, in a corner of every page — as Drive shows them.
// A finished one links to the file it became; a duplicate, to the file the
// user already has.

import { XIcon } from "lucide-react";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useT } from "@/i18n/I18nProvider";
import { type Upload, useUploads } from "./uploads";

export function UploadTray() {
  const { uploads, dismiss } = useUploads();
  const t = useT();
  if (uploads.length === 0) return null;
  const busy = uploads.some((row) => row.phase === "hashing" || row.phase === "uploading");
  return (
    <section
      aria-label={t.files.uploads.title}
      className="fixed right-4 bottom-4 z-40 w-80 rounded-xl border bg-popover p-3 shadow-lg"
    >
      <header className="mb-2 flex items-center justify-between">
        <h2 className="text-sm font-medium">{t.files.uploads.title}</h2>
        {!busy && (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t.files.uploads.clear}
            onClick={dismiss}
          >
            <XIcon />
          </Button>
        )}
      </header>
      <ul className="flex max-h-72 flex-col gap-3 overflow-y-auto">
        {uploads.map((row) => (
          <li key={row.key} className="flex flex-col gap-1 text-sm">
            <span className="truncate" title={row.name}>
              {row.name}
            </span>
            {row.phase === "hashing" || row.phase === "uploading" ? (
              <div className="flex items-center gap-2">
                <Progress value={Math.round(row.progress * 100)} className="flex-1" />
                <span className="w-20 text-right text-xs text-muted-foreground">
                  {t.files.uploads.phase[row.phase]}
                </span>
              </div>
            ) : (
              <Outcome row={row} />
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function Outcome({ row }: { row: Upload }) {
  const phase = useT().files.uploads.phase;
  if (row.phase !== "failed" && row.item !== undefined) {
    return (
      <Link to={`/library?item=${row.item}`} className="text-xs text-muted-foreground underline">
        {row.message ?? phase[row.phase]}
      </Link>
    );
  }
  const tone = row.phase === "failed" ? "text-destructive" : "text-muted-foreground";
  return <span className={`text-xs ${tone}`}>{row.message ?? phase[row.phase]}</span>;
}
