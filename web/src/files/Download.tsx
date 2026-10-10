// Saving a library file to the user's own disk (#711): a link to the file with
// `?download=1`, which the server answers as an attachment named for the file
// with its real extension. A plain link, not a fetch, so the browser streams a
// gigabyte straight to disk and shows its own progress, as a render's
// download does. Every kind downloads the same way.

import { DownloadIcon } from "lucide-react";
import { api } from "@/api";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

interface Props {
  file: { id: number; name: string };
}

/** The small icon button over a tile's corner. */
export function DownloadAction({ file }: Props) {
  const t = useT();
  return (
    <Button
      asChild
      size="icon-xs"
      variant="secondary"
      className="opacity-90 shadow-sm hover:opacity-100"
    >
      <a
        href={api.library.downloadUrl(file.id)}
        download
        aria-label={t.files.download.file(file.name)}
        title={t.files.download.file(file.name)}
        // The tile's own button selects, opens or picks; saving is none of them.
        onClick={(event) => event.stopPropagation()}
        onDoubleClick={(event) => event.stopPropagation()}
      >
        <DownloadIcon />
      </a>
    </Button>
  );
}

/** The labelled button in a file's details. */
export function DownloadButton({ file }: Props) {
  const t = useT();
  return (
    <Button asChild variant="secondary">
      <a href={api.library.downloadUrl(file.id)} download>
        <DownloadIcon /> {t.files.download.button}
      </a>
    </Button>
  );
}
