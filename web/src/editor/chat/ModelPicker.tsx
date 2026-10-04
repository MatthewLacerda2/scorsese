// Which model the project's assistant runs on (#705): a dropdown in the chat
// panel, written to the project on the server. A change while the
// conversation's cache may still be warm asks first (`switching.ts`). A model
// the server cannot reach is still listed, marked, with the server's own
// reason under the picker when it is the one chosen — never hidden.

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
import { SWITCH_WARNING, warnsBeforeSwitching } from "./switching";
import { problem } from "./transcript";

interface Props {
  projectId: number;
  model: string;
  models: ModelChoice[];
  turns: TurnView[];
}

export function ModelPicker({ projectId, model, models, turns }: Props) {
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
  const refused = choose.error ? problem(choose.error) : null;

  return (
    <div className="flex flex-col gap-1">
      <Select value={model} onValueChange={picked} disabled={choose.isPending}>
        <SelectTrigger size="sm" aria-label="Model">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {models.map((choice) => (
            <SelectItem key={choice.id} value={choice.id}>
              {choice.label}
              {choice.unavailable !== null && (
                <span className="text-muted-foreground"> · unavailable</span>
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
            <DialogTitle>Switch models?</DialogTitle>
            <DialogDescription>{SWITCH_WARNING}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAsking(null)}>
              Cancel
            </Button>
            <Button
              disabled={choose.isPending}
              onClick={() => asking !== null && choose.mutate(asking)}
            >
              Switch
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
