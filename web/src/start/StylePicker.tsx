// The kind of video a project is (#1016), filtered by the platform chosen and
// drawn the same in the new-project dialog and the editor's control.

import { cn } from "cn";
import { ClapperboardIcon } from "lucide-react";
import { useState } from "react";
import type { StyleView } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import type { Choosing } from "./choice/Choosing";
import { Group } from "./choice/Group";
import { Option } from "./choice/Option";
import { stylesFor } from "./menu";

/** The styles made for `platform` — every one with no platform — "none" first. */
export function StylePicker({
  menu,
  value,
  onChange,
  platform,
}: Choosing & { platform: string | null }) {
  const t = useT().start;
  return (
    <Group label={t.steps.style} hint={t.styleHint}>
      {platform === null && <p className="text-xs text-muted-foreground">{t.everyStyle}</p>}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 md:grid-cols-3">
        <Option checked={value === null} onPick={() => onChange(null)}>
          <span className="text-muted-foreground">{t.noStyle}</span>
        </Option>
        {stylesFor(menu, platform).map((style) => (
          <Option key={style.id} checked={value === style.id} onPick={() => onChange(style.id)}>
            <Preview style={style} />
            <span className="font-medium">{style.name}</span>
            <span className="text-xs text-muted-foreground">{style.description}</span>
          </Option>
        ))}
      </div>
    </Group>
  );
}

/**
 * A style's moving preview — or, until it has one (#1017) or when it fails to
 * load, a neutral card: never a broken image.
 */
function Preview({ style }: { style: StyleView }) {
  const t = useT().start;
  const [failed, setFailed] = useState(false);
  const frame = "aspect-video w-full rounded-md bg-muted object-cover";
  if (!style.preview || failed)
    return (
      <span
        data-testid="style-placeholder"
        className={cn(
          frame,
          "flex flex-col items-center justify-center gap-1 text-muted-foreground",
        )}
      >
        <ClapperboardIcon className="size-5" />
        <span className="text-xs">{t.noPreview}</span>
      </span>
    );
  if (style.preview.endsWith(".mp4"))
    return (
      <video
        src={style.preview}
        className={frame}
        autoPlay
        muted
        loop
        playsInline
        onError={() => setFailed(true)}
      />
    );
  return <img src={style.preview} alt="" className={frame} onError={() => setFailed(true)} />;
}
