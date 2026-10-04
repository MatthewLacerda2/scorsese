// Which model the project's assistant runs on (#705): a dropdown in the chat
// panel, written to the project on the server. A change while the
// conversation's cache may still be warm asks first (`switching.ts`). A model
// the server cannot reach is still listed, marked, with the server's own
// reason under the picker when it is the one chosen — never hidden. Each model
// carries a green-to-red bar for how dear it is beside the others (#706).

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { chatApi, type ModelChoice, type TurnView } from "@/api/chat";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useT } from "@/i18n/I18nProvider";
import { colour, describe, position } from "./cost";
import { warnsBeforeSwitching } from "./switching";
import { problem } from "./transcript";

interface Props {
  projectId: number;
  model: string;
  models: ModelChoice[];
  turns: TurnView[];
}

export function ModelPicker({ projectId, model, models, turns }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const [asking, setAsking] = useState<string | null>(null);
  const choose = useMutation({
    mutationFn: (next: string) => chatApi.chooseModel(projectId, next),
    onSettled: () => {
      setAsking(null);
      queryClient.invalidateQueries({ queryKey: ["chat", projectId] });
    },
  });
  const current = models.find((choice) => choice.id === model);
  const picked = (next: string) => {
    if (next === model) return;
    if (warnsBeforeSwitching(turns, current, Date.now() / 1000)) setAsking(next);
    else choose.mutate(next);
  };
  const refused = choose.error ? problem(choose.error, t.chat) : null;

  return (
    <div className="flex flex-col gap-1">
      <Select value={model} onValueChange={picked} disabled={choose.isPending}>
        <SelectTrigger size="sm" aria-label={t.chat.model.label}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {models.map((choice) => (
            <SelectItem key={choice.id} value={choice.id}>
              {choice.label}
              {choice.cost !== null && <CostBar cost={choice.cost} models={models} />}
              {choice.unavailable !== null && (
                <span className="text-muted-foreground"> · {t.chat.model.unavailable}</span>
              )}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {current?.unavailable && <p className="text-xs text-destructive">{current.unavailable}</p>}
      {refused && <p className="text-xs text-destructive">{refused.detail}</p>}
      <Dialog open={asking !== null} onOpenChange={(open) => !open && setAsking(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.chat.model.switchTitle}</DialogTitle>
            <DialogDescription>{t.chat.model.switchWarning}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAsking(null)}>
              {t.common.cancel}
            </Button>
            <Button
              disabled={choose.isPending}
              onClick={() => asking !== null && choose.mutate(asking)}
            >
              {t.chat.model.switch}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

/** How dear a model is beside the others: a bar, never a price. */
function CostBar({ cost, models }: { cost: number; models: ModelChoice[] }) {
  const t = useT();
  const at = position(cost, models);
  const words = describe(at, t.chat.cost);
  return (
    <span
      role="img"
      aria-label={t.chat.model.cost(words)}
      title={words}
      className="ml-auto h-1.5 w-12 shrink-0 overflow-hidden rounded-full bg-muted"
    >
      <span
        className="block h-full rounded-full"
        style={{ width: `${cost}%`, backgroundColor: colour(at) }}
      />
    </span>
  );
}
