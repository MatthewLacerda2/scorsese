// The rows of the spending history. A generation that made a file links to
// it in the library; one the provider failed says it was free, and why.

import { Link } from "react-router";
import type { HistoryKind, HistoryRow } from "@/api";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { formatDollars, formatMovement } from "@/lib/money";

export const KIND_LABEL: Record<HistoryKind, string> = {
  veo_shot: "Video generation",
  spoken_line: "Speech generation",
  assistant: "Assistant",
  top_up: "Top-up",
  monthly_fee: "Monthly fee",
  refund: "Refund",
};

const STATUS: Record<HistoryRow["status"], { label: string; tone: "secondary" | "outline" }> = {
  charged: { label: "Charged", tone: "secondary" },
  free: { label: "Free: the provider failed", tone: "outline" },
  pending: { label: "Pending", tone: "outline" },
  credited: { label: "Credited", tone: "secondary" },
};

export function HistoryTable({ rows }: { rows: HistoryRow[] }) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>When</TableHead>
          <TableHead>What</TableHead>
          <TableHead className="text-right">Amount</TableHead>
          <TableHead className="text-right">Balance after</TableHead>
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
              <div>{formatMovement(row.amount_micros, row.amount_centavos)}</div>
              {row.amount_centavos !== null && row.amount_micros !== 0 && (
                <div className="text-xs text-muted-foreground">
                  {formatDollars(row.amount_micros)}
                </div>
              )}
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
  const status = STATUS[row.status];
  return (
    <div className="flex flex-col gap-1">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{KIND_LABEL[row.kind]}</span>
        <Badge variant={status.tone}>{status.label}</Badge>
      </div>
      {row.memo && row.memo !== KIND_LABEL[row.kind] && <span className="text-sm">{row.memo}</span>}
      <span className="text-xs text-muted-foreground">
        {row.project_name ?? (row.project_id !== null ? "a deleted project" : null)}
        {typeof item === "number" && (
          <>
            {row.project_id !== null && " · "}
            <Link to={`/library?item=${item}`} className="underline">
              the file it made
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
