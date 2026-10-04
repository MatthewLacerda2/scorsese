// The rows of the spending history. A generation that made a file links to
// it in the library; one the provider failed says it was free, and why.

import { Link } from "react-router";
import type { HistoryRow } from "@/api";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { en } from "@/i18n/en";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars, formatMovement } from "@/lib/money";

const TONE: Record<HistoryRow["status"], "secondary" | "outline"> = {
  charged: "secondary",
  free: "outline",
  pending: "outline",
  credited: "secondary",
};

export function HistoryTable({ rows }: { rows: HistoryRow[] }) {
  const words = useT().pages.history;
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{words.when}</TableHead>
          <TableHead>{words.what}</TableHead>
          <TableHead className="text-right">{words.amount}</TableHead>
          <TableHead className="text-right">{words.balanceAfter}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.id}>
            <TableCell className="align-top whitespace-nowrap text-muted-foreground">
              {row.when}
            </TableCell>
            <TableCell className="align-top whitespace-normal">
              <What row={row} />
            </TableCell>
            <TableCell className="text-right align-top whitespace-nowrap">
              {formatMovement(row.amount_micros)}
            </TableCell>
            <TableCell className="text-right align-top whitespace-nowrap text-muted-foreground">
              {formatDollars(row.balance_after_micros)}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function What({ row }: { row: HistoryRow }) {
  const item = row.detail.library_item_id;
  const error = row.detail.error;
  const words = useT().pages.history;
  // The memo is the server's, in English; one that only repeats the kind's
  // English name says nothing the label above it does not.
  const repeatsKind = row.memo === en.pages.history.kinds[row.kind];
  return (
    <div className="flex flex-col gap-1">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{words.kinds[row.kind]}</span>
        <Badge variant={TONE[row.status]}>{words.status[row.status]}</Badge>
      </div>
      {row.memo && !repeatsKind && <span className="text-sm">{row.memo}</span>}
      <span className="text-xs text-muted-foreground">
        {row.project_name ?? (row.project_id !== null ? words.deletedProject : null)}
        {typeof item === "number" && (
          <>
            {row.project_id !== null && " · "}
            <Link to={`/library?item=${item}`} className="underline">
              {words.fileItMade}
            </Link>
          </>
        )}
      </span>
      {row.status === "free" && typeof error === "string" && (
        <span className="text-xs text-muted-foreground">{error}</span>
      )}
    </div>
  );
}
