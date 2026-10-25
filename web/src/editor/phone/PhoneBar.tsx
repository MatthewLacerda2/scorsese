// The editor's bar on a phone (#1570), in place of the app header: the menu
// on the left (the sidebar's drawer: the library, the credits, settings, the
// home), and on the right the switch between the chat and the video — a
// clapperboard to the video, a chat bubble back. In the video view the bar
// also holds Render, since that is where someone decides the cut looks right.

import { ClapperboardIcon, MessageCircleIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SidebarSheet } from "@/home/SidebarSheet";
import { useT } from "@/i18n/I18nProvider";
import { EditorActions, type EditorActionsProps } from "../EditorActions";

interface Props extends EditorActionsProps {
  /** Whether the video view is showing, rather than the chat. */
  video: boolean;
  onVideo: (video: boolean) => void;
}

export function PhoneBar({ video, onVideo, ...actions }: Props) {
  const t = useT().editor.page;
  const label = video ? t.showChat : t.showVideo;
  return (
    <header className="flex items-center gap-2 border-b px-2 py-1.5">
      <SidebarSheet />
      {video ? (
        <EditorActions {...actions} />
      ) : (
        <h1 className="min-w-0 flex-1 truncate font-heading font-semibold" title={actions.name}>
          {actions.name}
        </h1>
      )}
      <Button
        size="icon"
        variant="ghost"
        className="shrink-0"
        aria-label={label}
        title={label}
        onClick={() => onVideo(!video)}
      >
        {video ? <MessageCircleIcon /> : <ClapperboardIcon />}
      </Button>
    </header>
  );
}
