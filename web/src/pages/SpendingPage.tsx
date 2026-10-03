// The spending history (#537): one list of everything that moved the balance —
// generations, assistant turns, top-ups, fees, refunds — each with its amount
// and the balance after it, filterable by project, kind and date, with the
// total for the filter. A list, deliberately: no charts (#544, out of scope).

import { useInfiniteQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api, HISTORY_KINDS, type HistoryFilter, type HistoryKind } from "@/api";
import { useProjects } from "@/app/queries";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatDollars } from "@/lib/money";
import { HistoryTable, KIND_LABEL } from "./HistoryTable";

/** Rows fetched per page; "Show more" asks for the next. */
const PAGE = 100;
const ALL = "all";

export function SpendingPage() {
  const [filter, setFilter] = useState<Omit<HistoryFilter, "before" | "limit">>({});
  const history = useInfiniteQuery({
    queryKey: ["credits", "history", filter],
    queryFn: ({ pageParam }) => api.credits.history({ ...filter, before: pageParam, limit: PAGE }),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (last) => (last.rows.length === PAGE ? last.rows.at(-1)?.id : undefined),
  });
  const first = history.data?.pages[0];
  const rows = history.data?.pages.flatMap((page) => page.rows) ?? [];

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="font-heading text-2xl font-semibold">Spending history</h1>
        {first && (
          <p className="text-sm">
            Balance <span className="font-medium">{formatDollars(first.balance_micros)}</span>
          </p>
        )}
      </div>
      <Filters filter={filter} onChange={setFilter} />
      {first && (
        <p className="text-sm text-muted-foreground">
          {first.matched} {first.matched === 1 ? "entry" : "entries"}, adding up to{" "}
          <span className="font-medium text-foreground">{formatDollars(first.total_micros)}</span>.
        </p>
      )}
      {history.isError && <p className="text-destructive">{history.error.message}</p>}
      {history.isPending && <p className="text-muted-foreground">Loading…</p>}
      {history.isSuccess && rows.length === 0 && (
        <p className="text-muted-foreground">Nothing has moved your balance yet.</p>
      )}
      {rows.length > 0 && <HistoryTable rows={rows} />}
      {history.hasNextPage && (
        <Button
          variant="outline"
          className="self-center"
          disabled={history.isFetchingNextPage}
          onClick={() => history.fetchNextPage()}
        >
          Show more
        </Button>
      )}
    </div>
  );
}

type Filter = Omit<HistoryFilter, "before" | "limit">;

function Filters({ filter, onChange }: { filter: Filter; onChange: (next: Filter) => void }) {
  const projects = useProjects();
  const set = (change: Filter) => onChange({ ...filter, ...change });
  return (
    <div className="flex flex-wrap items-end gap-3">
      <div className="flex flex-col gap-1">
        <Label htmlFor="spending-project">Project</Label>
        <Select
          value={filter.project === undefined ? ALL : String(filter.project)}
          onValueChange={(value) => set({ project: value === ALL ? undefined : Number(value) })}
        >
          <SelectTrigger id="spending-project" className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>All projects</SelectItem>
            {projects.data?.map((project) => (
              <SelectItem key={project.id} value={String(project.id)}>
                {project.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="spending-kind">Kind</Label>
        <Select
          value={filter.kind ?? ALL}
          onValueChange={(value) =>
            set({ kind: value === ALL ? undefined : (value as HistoryKind) })
          }
        >
          <SelectTrigger id="spending-kind" className="w-44">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>Everything</SelectItem>
            {HISTORY_KINDS.map((kind) => (
              <SelectItem key={kind} value={kind}>
                {KIND_LABEL[kind]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="spending-since">From (UTC)</Label>
        <Input
          id="spending-since"
          type="date"
          value={filter.since ?? ""}
          onChange={(event) => set({ since: event.target.value || undefined })}
        />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="spending-until">To (UTC)</Label>
        <Input
          id="spending-until"
          type="date"
          value={filter.until ?? ""}
          onChange={(event) => set({ until: event.target.value || undefined })}
        />
      </div>
      {Object.values(filter).some((value) => value !== undefined) && (
        <Button variant="ghost" onClick={() => onChange({})}>
          Clear filters
        </Button>
      )}
    </div>
  );
}
