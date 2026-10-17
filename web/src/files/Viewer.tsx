// Opening a file: the picture, or the video or sound playing. The browser
// streams it from `GET /api/library/{id}/file` in ranges, so a long video
// starts at once and seeks without being fetched whole.

import type { FileKind } from "@/api";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { Media } from "./Media";

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
