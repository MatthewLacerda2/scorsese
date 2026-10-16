// The editor's controls in the app header (#764): `EditorActions`, drawn in
// the header's slot (`HeaderSlot`) rather than in a strip of the editor's own,
// which took height the timeline needs while the header sat half empty.

import { HeaderSlot } from "@/app/headerSlot";
import { EditorActions, type EditorActionsProps } from "./EditorActions";

/** The editor's controls, drawn in the app header. */
export function EditorHeader(props: EditorActionsProps) {
  return (
    <HeaderSlot>
      <EditorActions {...props} />
    </HeaderSlot>
  );
}
