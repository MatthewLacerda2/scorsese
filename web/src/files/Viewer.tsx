// Opening a file: the picture, or the video or sound playing. The browser
// streams it from `GET /api/library/{id}/file` in ranges, so a long video
// starts at once and seeks without being fetched whole.

import { api, type FileKind } from "@/api";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";

export interface Opened {
  id: number;
  name: string;
  kind: FileKind;
}

export function Viewer({ file, onClose }: { file: Opened | null; onClose: () => void }) {
  return (
    <Dialog open={file !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-4xl">
        {file && (
          <>
            <DialogTitle className="truncate pr-8">{file.name}</DialogTitle>
            <Media file={file} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function Media({ file }: { file: Opened }) {
  const src = api.library.fileUrl(file.id);
  switch (file.kind) {
    case "image":
      return <img src={src} alt={file.name} className="max-h-[75vh] w-full object-contain" />;
    case "video":
      // biome-ignore lint/a11y/useMediaCaption: a user's own footage has no caption track to offer.
      return <video src={src} controls autoPlay className="max-h-[75vh] w-full bg-black" />;
    case "audio":
      // biome-ignore lint/a11y/useMediaCaption: a user's own recording has no caption track to offer.
      return <audio src={src} controls autoPlay className="w-full" />;
  }
}
