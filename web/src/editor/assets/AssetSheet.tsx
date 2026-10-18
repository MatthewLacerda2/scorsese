// The side panel an asset opens in when its tile is clicked (#1446): the
// library page's own `Sheet`, from the right with its blurred overlay, and its
// own `Media` for the picture or the playing video or sound. Read-only — a
// generated asset's state and brief are shown, and changing them stays a
// sentence to the assistant. A project file *is* a library item, found by its
// hash as the tile's thumbnail is (`AssetTile`).

import { useQuery } from "@tanstack/react-query";
import { api, type DocumentAsset } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Media } from "@/files/Media";
import { useT } from "@/i18n/I18nProvider";
import { kindName } from "./kinds";

interface Props {
  asset: DocumentAsset | null;
  onClose: () => void;
}

export function AssetSheet({ asset, onClose }: Props) {
  return (
    <Sheet open={asset !== null} onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        {asset && <Details asset={asset} />}
      </SheetContent>
    </Sheet>
  );
}

function Details({ asset }: { asset: DocumentAsset }) {
  const t = useT();
  const name = asset.text ?? asset.id;
  const file = useQuery({
    queryKey: queryKeys.libraryList({ sha256: asset.sha256 }),
    queryFn: () => api.library.list({ sha256: asset.sha256 }),
    enabled: asset.sha256 !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  }).data?.[0];
  const brief = asset.prompt ?? asset.recipe;
  const state = asset.state;
  return (
    <div className="flex flex-col gap-4 px-4 pb-6">
      <SheetHeader className="px-0">
        <SheetTitle className="truncate">{name}</SheetTitle>
        <SheetDescription>{kindName(asset.kind, t.assets.kinds)}</SheetDescription>
      </SheetHeader>
      {file && <Media file={file} />}
      {state && (
        <section className="flex flex-col gap-1 text-sm">
          <h3 className="font-medium">{t.editor.states[state] ?? state}</h3>
          <p className="text-muted-foreground">{t.inspector.brief.states[state]}</p>
        </section>
      )}
      {brief && (
        <section className="flex flex-col gap-1 text-sm">
          <h3 className="font-medium">{t.inspector.brief.title}</h3>
          <p className="whitespace-pre-wrap rounded-md bg-muted p-3">{brief}</p>
        </section>
      )}
    </div>
  );
}
