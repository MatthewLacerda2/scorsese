// The login page. There is no sign-up: the operator creates accounts and
// hands out the first password (docs/web.md, *Accounts*), so this page says so
// instead of offering a form nobody can use.

import { type FormEvent, useState } from "react";
import { Navigate, useSearchParams } from "react-router";
import { ThemeToggle } from "@/app/ThemeToggle";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { safeNext, useAccount, useLogin } from "@/session/session";

export function LoginPage() {
  const [params] = useSearchParams();
  const next = safeNext(params.get("next"));
  const account = useAccount();
  const login = useLogin();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  if (account.data) return <Navigate to={next} replace />;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    login.mutate({ email, password });
  };

  return (
    <main className="relative flex min-h-svh items-center justify-center p-6">
      <ThemeToggle className="absolute top-4 right-4" />
      <Card className="w-full max-w-sm">
        <CardHeader className="justify-items-center text-center">
          <img
            src="/logo.png"
            alt=""
            width={210}
            height={256}
            className="mx-auto mb-2 h-32 w-auto rounded-xl"
          />
          <CardTitle className="font-heading text-2xl">scorsese</CardTitle>
          <CardDescription>Log in to your projects and files.</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="flex flex-col gap-4" onSubmit={submit}>
            <div className="flex flex-col gap-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                autoComplete="username"
                required
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
            </div>
            {login.isError && (
              <p role="alert" className="text-sm text-destructive">
                {login.error.message}
              </p>
            )}
            <Button type="submit" disabled={login.isPending}>
              {login.isPending ? "Logging in…" : "Log in"}
            </Button>
            <p className="text-xs text-muted-foreground">
              Accounts are made by invitation. Ask whoever gave you access for a password.
            </p>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
