// Low, medium, high or extra, beside the model picker (#1443): how hard the
// assistant thinks, kept on the project on the server, with one line of hover
// text each saying what it is for. A change takes effect from the next
// message and costs no cache (`effort.ts` has the rest).

import { chatApi } from "@/api/chat";
import { queryKeys } from "@/app/queryKeys";
import { useInvalidatingMutation } from "@/app/useInvalidatingMutation";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useT } from "@/i18n/I18nProvider";
import { EFFORTS, type Effort } from "./effort";
import { problem } from "./transcript";

export function EffortPicker({ projectId, effort }: { projectId: number; effort: Effort }) {
  const t = useT();
  const choose = useInvalidatingMutation([queryKeys.conversation(projectId)], {
    mutationFn: (next: Effort) => chatApi.chooseEffort(projectId, next),
  });
  const shown = choose.isPending && choose.variables ? choose.variables : effort;
  const refused = choose.error ? problem(choose.error, t.chat) : null;
  return (
    <div className="flex shrink-0 flex-col gap-1">
      <Select
        value={shown}
        onValueChange={(value) => value !== effort && choose.mutate(value as Effort)}
        disabled={choose.isPending}
      >
        <SelectTrigger size="sm" aria-label={t.chat.effort.label} title={t.chat.effort.hint[shown]}>
          <SelectValue>{t.chat.effort.name[shown]}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          {EFFORTS.map((level) => (
            <SelectItem key={level} value={level} title={t.chat.effort.hint[level]}>
              {t.chat.effort.name[level]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {refused && <p className="text-xs text-destructive">{refused.detail}</p>}
    </div>
  );
}
