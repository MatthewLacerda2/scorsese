// The sign-in popup (#904): the login form that was the whole of `/login`,
// now a dialog over the landing page. Same `POST /api/login`, same errors,
// and the login brake's `429` sentence shown as the server words it
// (docs/web.md, *The login's brake*). There is no sign-up: the operator makes
// accounts (docs/web.md, *Accounts*), and a second way in (Google, planned
// separately) would sit beside the form as a section of its own.
//
// It wears the app's dark tokens (`dark` on the content), whatever theme the
// app is set to, because it opens over the landing page's own dark art
// direction.

import { type FormEvent, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useT } from "@/i18n/I18nProvider";
import { useLogin } from "@/session/session";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SignInDialog({ open, onOpenChange }: Props) {
  const t = useT();
  const login = useLogin();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    login.mutate({ email, password });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="dark gap-6 bg-[#0e0d12] p-8 text-[#f4efe6] ring-[#f4efe6]/10 sm:max-w-md"
        overlayClassName="bg-black/60 supports-backdrop-filter:backdrop-blur-sm"
      >
        <DialogHeader className="gap-2">
          <img src="/icon.png" alt="" width={48} height={48} className="mb-2 size-12 rounded-xl" />
          <DialogTitle
            className="text-3xl font-semibold tracking-tight"
            style={{ fontFamily: '"Playfair Display", serif' }}
          >
            {t.landing.dialog.title}
          </DialogTitle>
          <DialogDescription>{t.landing.dialog.description}</DialogDescription>
        </DialogHeader>
        <form className="flex flex-col gap-4" onSubmit={submit}>
          <div className="flex flex-col gap-2">
            <Label htmlFor="email">{t.common.login.email}</Label>
            <Input
              id="email"
              type="email"
              autoComplete="username"
              required
              className="h-10"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="password">{t.common.login.password}</Label>
            <Input
              id="password"
              type="password"
              autoComplete="current-password"
              required
              className="h-10"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </div>
          {login.isError && (
            <p role="alert" className="text-sm text-destructive">
              {login.error.message}
            </p>
          )}
          <Button
            type="submit"
            disabled={login.isPending}
            className="h-11 rounded-full bg-[#f2b134] text-[#07070a] hover:bg-[#f2b134]/90"
          >
            {login.isPending ? t.common.login.submitting : t.common.login.submit}
          </Button>
          <p className="text-xs text-muted-foreground">{t.common.login.invitation}</p>
        </form>
      </DialogContent>
    </Dialog>
  );
}
