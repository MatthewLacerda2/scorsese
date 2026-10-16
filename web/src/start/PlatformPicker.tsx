// Where a project's video will be placed (#1016), drawn the same in the
// new-project dialog and the editor's control.

import type { PlatformView } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import type { Choosing } from "./choice/Choosing";
import { Group } from "./choice/Group";
import { Option } from "./choice/Option";

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

/** `1080×1920`. */
function size(platform: PlatformView) {
  return `${platform.width}×${platform.height}`;
}
