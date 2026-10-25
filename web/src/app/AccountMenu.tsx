// Who is signed in, and the way out: the round button at the end of the
// shell's header. The menu names the person as their Google account does
// (#1266), with the email under it; an account Google gave no name shows the
// email alone.

import { useNavigate } from "react-router";
import { Mark } from "@/components/Mark";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useT } from "@/i18n/I18nProvider";
import { useAccount, useLogout } from "@/session/session";

export function AccountMenu() {
  const account = useAccount();
  const logout = useLogout();
  const navigate = useNavigate();
  const t = useT();
  const name = account.data?.name;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="rounded-full" aria-label={t.common.account}>
          <Mark kind="icon" size={28} className="size-7 rounded-full" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel className="flex flex-col font-normal">
          {name && <span>{name}</span>}
          <span className="text-muted-foreground">{account.data?.email}</span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => navigate("/spending")}>
          {t.common.spendingHistory}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={() => logout.mutate(undefined, { onSettled: () => navigate("/login") })}
        >
          {t.common.logOut}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
