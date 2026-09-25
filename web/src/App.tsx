// A placeholder page, and deliberately nothing more: the real pages (login,
// the library, the editor) are their own issues under #527. It uses one
// shadcn/ui component so the whole chain — Tailwind v4, the theme variables,
// the `@/` alias, a generated component — is proven to build before any of
// those pages depend on it.

import { Button } from "@/components/ui/button";

export function App() {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-4 p-6">
      <h1 className="font-heading text-3xl font-semibold">scorsese</h1>
      <p className="text-muted-foreground">The web app is under construction.</p>
      <Button asChild variant="outline">
        <a href="https://github.com/MatthewLacerda2/scorsese">Follow along</a>
      </Button>
    </main>
  );
}
