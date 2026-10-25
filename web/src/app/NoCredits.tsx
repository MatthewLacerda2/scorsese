// The plain line for a balance of zero (#1244): a new account starts there,
// and a paid action is refused until it has credits. It points at the same
// Add credits action as the header's balance.

import { useAddCredits } from "@/app/addCredits";
import { useBalance } from "@/app/queries";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

/** `always` shows it whatever the balance is: the caller knows a charge was refused. */
export function NoCredits({ always = false }: { always?: boolean }) {
  const balance = useBalance();
  const addCredits = useAddCredits();
  const t = useT();
  // Free credit left today is credit too (#1530).
  const spendable = balance.data && balance.data.balance_micros + (balance.data.free_micros ?? 0);
  if (!always && !(spendable !== undefined && spendable <= 0)) return null;
  return (
    <p className="text-sm text-muted-foreground">
      {t.buying.empty}{" "}
      <Button variant="link" className="h-auto p-0" onClick={addCredits}>
        {t.buying.add}
      </Button>
    </p>
  );
}
