// The balance in the header (#1280). Clicking it opens a small modal with the
// balance, the "Add credits" button and a link to the spending history.

import { useState } from "react";
import { Link } from "react-router";
import { useAddCredits } from "@/app/addCredits";
import { useBalance } from "@/app/queries";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";

export function BalanceChip() {
  const balance = useBalance();
  const addCredits = useAddCredits();
  const t = useT();
  const [open, setOpen] = useState(false);
  if (!balance.data) return null;
  const { balance_micros } = balance.data;
  const dollars = formatDollars(balance_micros);
  return (
    <>
      <Button
        variant="outline"
        size="sm"
        title={t.common.balanceTitle}
        onClick={() => setOpen(true)}
      >
        <span className={balance_micros < 0 ? "text-destructive" : undefined}>{dollars}</span>
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-xs">
          <DialogHeader>
            <DialogTitle>{t.common.yourCredits}</DialogTitle>
            <DialogDescription className="font-heading text-3xl font-semibold text-foreground">
              {dollars}
            </DialogDescription>
          </DialogHeader>
          <Button
            onClick={() => {
              setOpen(false);
              addCredits();
            }}
          >
            {t.buying.add}
          </Button>
          <Button asChild variant="link">
            <Link to="/spending" onClick={() => setOpen(false)}>
              {t.common.spendingHistory}
            </Link>
          </Button>
        </DialogContent>
      </Dialog>
    </>
  );
}
