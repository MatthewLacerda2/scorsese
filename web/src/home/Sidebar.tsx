// The home's sidebar, top to bottom: the name, the library, the credits, the
// projects (which scroll on their own), then settings, spending and the way
// out. The same content sits fixed beside the page on a wide screen and inside
// a sheet behind a menu button on a narrow one.

import { FolderIcon, LogOutIcon, ReceiptIcon } from "lucide-react";
import { Link, useNavigate } from "react-router";
import { BalanceChip } from "@/app/BalanceChip";
import { useProjects } from "@/app/queries";
import { SettingsButton } from "@/app/SettingsButton";
import { Mark } from "@/components/Mark";
import { Button } from "@/components/ui/button";
import { QueryError } from "@/components/ui/query-error";
import { ProjectItem } from "@/home/ProjectItem";
import { useT } from "@/i18n/I18nProvider";
import { useLogout } from "@/session/session";

export function Sidebar({ onNavigate }: { onNavigate?: () => void }) {
  const t = useT();
  const projects = useProjects();
  const logout = useLogout();
  const navigate = useNavigate();
  return (
    <nav aria-label={t.home.nav} className="flex h-full min-h-0 flex-col gap-3 p-3">
      <Link
        to="/"
        onClick={onNavigate}
        className="display flex items-center gap-2 px-2.5 pt-1 font-heading text-xl font-semibold tracking-tight"
      >
        <Mark kind="icon" size={28} className="size-7 rounded-md" />
        scorsese
      </Link>
      <Button asChild variant="ghost" className="w-full justify-start">
        <Link to="/library" onClick={onNavigate}>
          <FolderIcon /> {t.common.nav.library}
        </Link>
      </Button>
      <div className="px-1">
        <BalanceChip />
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-1">
        <h2 className="px-2.5 pt-2 text-xs font-medium text-muted-foreground">{t.home.projects}</h2>
        <QueryError error={projects.error} className="px-2.5 text-xs text-destructive" />
        {projects.data?.length === 0 && (
          <p className="px-2.5 text-sm text-muted-foreground">{t.home.noProjects}</p>
        )}
        <ul className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
          {projects.data?.map((project) => (
            <ProjectItem key={project.id} project={project} onNavigate={onNavigate} />
          ))}
        </ul>
      </div>
      <div className="flex flex-col gap-0.5 border-t pt-3">
        <SettingsButton />
        <Button asChild variant="ghost" className="w-full justify-start">
          <Link to="/spending" onClick={onNavigate}>
            <ReceiptIcon /> {t.common.spendingHistory}
          </Link>
        </Button>
        <Button
          variant="ghost"
          className="w-full justify-start"
          onClick={() => logout.mutate(undefined, { onSettled: () => navigate("/login") })}
        >
          <LogOutIcon /> {t.common.logOut}
        </Button>
      </div>
    </nav>
  );
}
