// The two choices a project is started for (#1016), drawn the same in the
// new-project dialog and the editor's control: a placement, and a kind of
// video filtered by it. Each is a radio group whose first option is "none",
// because both are optional and a person with a clear idea skips them.

import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { ClapperboardIcon } from "lucide-react";
import { type ReactNode, useCallback, useState } from "react";
import { api, type PlatformView, type StyleMenu, type StyleView } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import { localized, stylesFor } from "./menu";

/**
 * The menu, read once — it is compiled into the server and never changes
 * under a page — and shown in the page's language.
 */
export function useStyleMenu() {
  const words = useT().menu;
  const select = useCallback((menu: StyleMenu) => localized(menu, words), [words]);
  return useQuery({ queryKey: ["styles"], queryFn: api.styles.menu, staleTime: Infinity, select });
}

interface Choosing {
  menu: StyleMenu;
  value: string | null;
  onChange: (id: string | null) => void;
}

/** The placements, "none" first. */
export function PlatformPicker({ menu, value, onChange }: Choosing) {
  const t = useT().start;
  return (
    <Group label={t.steps.platform} hint={t.platformHint}>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <Option checked={value === null} onPick={() => onChange(null)}>
          <span className="text-muted-foreground">{t.noPlatform}</span>
        </Option>
        {menu.platforms.map((platform) => (
          <Option
            key={platform.id}
            checked={value === platform.id}
            onPick={() => onChange(platform.id)}
          >
            <span className="font-medium">{platform.name}</span>
            <span className="text-xs text-muted-foreground">{size(platform)}</span>
          </Option>
        ))}
      </div>
    </Group>
  );
}

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

/** A choice's heading, the line saying it can change later, and its options. */
function Group({ label, hint, children }: { label: string; hint: string; children: ReactNode }) {
  return (
    <div role="radiogroup" aria-label={label} className="flex flex-col gap-2">
      <p className="text-sm text-muted-foreground">{hint}</p>
      {children}
    </div>
  );
}

function Option({
  checked,
  onPick,
  children,
}: {
  checked: boolean;
  onPick: () => void;
  children: ReactNode;
}) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a card with a picture in it, not a bare radio input.
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      onClick={onPick}
      className={cn(
        "flex flex-col items-start gap-1 rounded-lg border p-2 text-left text-sm transition-colors hover:bg-muted/50",
        checked && "border-primary ring-2 ring-primary/30",
      )}
    >
      {children}
    </button>
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

/** `1080×1920`. */
function size(platform: PlatformView) {
  return `${platform.width}×${platform.height}`;
}
