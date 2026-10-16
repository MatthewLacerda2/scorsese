// The sign-in popup (#904), now one way in: "Continue with Google" (#952),
// which is sign-up too — a first sign-in makes the account (docs/web.md,
// *Accounts*). The button is a plain link to `GET /api/auth/google`, because
// the flow is a run of full-page redirects (to Google and back), not a fetch.
// It carries `?next=` through, so the server lands the person where the
// session guard sent them from.
//
// A sign-in that went wrong comes back to `/login?error=<why>`; the popup says
// that in words, from the catalogue, never the code itself.
//
// Signing in is also agreeing to the terms and the privacy policy (#547), so
// the popup says so under the button, linking both in a new tab: a first
// sign-in makes the account, and there is no other step to put it on.
//
// It wears the app's dark tokens (`dark` on the content), whatever theme the
// app is set to, because it opens over the landing page's own dark art
// direction.

import { Link, useSearchParams } from "react-router";
import { api } from "@/api";
import { buttonVariants } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useT } from "@/i18n/I18nProvider";
import { cn } from "@/lib/utils";
import { safeNext } from "@/session/session";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Where the button sends the browser: the server's start of the flow. */
function googleSignIn(next: string | null): string {
  return api.account.googleSignIn(next === null ? null : safeNext(next));
}

export function SignInDialog({ open, onOpenChange }: Props) {
  const t = useT();
  const [params] = useSearchParams();
  const errors = t.common.login.errors;
  const accept = t.common.login.accept;
  const code = params.get("error");
  const error = code && Object.hasOwn(errors, code) ? errors[code as keyof typeof errors] : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="dark gap-6 bg-ink-raised p-8 text-paper ring-paper/10 sm:max-w-md"
        overlayClassName="bg-screen/60 supports-backdrop-filter:backdrop-blur-sm"
      >
        <DialogHeader className="gap-2">
          <img src="/icon.png" alt="" width={48} height={48} className="mb-2 size-12 rounded-xl" />
          <DialogTitle className="text-3xl font-semibold tracking-tight">
            {t.landing.dialog.title}
          </DialogTitle>
          <DialogDescription>{t.landing.dialog.description}</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          {error && (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          )}
          <a
            href={googleSignIn(params.get("next"))}
            className={cn(buttonVariants(), "h-11 rounded-full bg-gold text-ink hover:bg-gold/90")}
          >
            {t.common.login.google}
          </a>
          <p className="text-xs text-muted-foreground">{t.common.login.newHere}</p>
          <p className="text-xs text-muted-foreground">
            {accept.before}
            <Link to="/terms" target="_blank" className="underline">
              {accept.terms}
            </Link>
            {accept.between}
            <Link to="/privacy" target="_blank" className="underline">
              {accept.privacy}
            </Link>
            {accept.after}
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
