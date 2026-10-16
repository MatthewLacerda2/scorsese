// Saving a library file to the user's own disk (#711), from its details: the
// same plain link as a tile's corner button, `DownloadAction.tsx` says why.

import { DownloadIcon } from "lucide-react";
import { api } from "@/api";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

interface Props {
  file: { id: number; name: string };
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
