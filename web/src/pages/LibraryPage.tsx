// Everything in the library (#544): the file browser, unnarrowed.
// `ProjectFilesPage` is the same browser narrowed to one project's files.

import { FileBrowser } from "@/files/FileBrowser";
import { useT } from "@/i18n/I18nProvider";

export function LibraryPage() {
  const t = useT();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="font-heading text-2xl font-semibold">{t.common.nav.library}</h1>
      <FileBrowser />
    </div>
  );
}
