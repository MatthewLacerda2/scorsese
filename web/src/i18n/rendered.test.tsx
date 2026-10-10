// A page renders in the language it is given, all of it — the shell, the
// page, the dollars left as dollars. Through `react-dom/server`, like
// App.test.tsx: there is no DOM to click the Settings select in yet, so the
// choice is passed in, which is what the provider does with a click.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { App } from "@/App";
import { createQueryClient, ME } from "@/app/queryClient";
import type { Language } from "@/i18n/language";
import { prerenderHtml } from "@/test/prerender";

function render(url: string, language: Language, signedIn: boolean) {
  const client = createQueryClient();
  client.setQueryData(ME, signedIn ? { id: 1, email: "ana@example.com", created_at: 0 } : null);
  client.setQueryData(["credits", "balance"], { balance_micros: 1_234_560_000 });
  client.setQueryData(["library", "list", {}], []);
  return renderToString(
    <App
      language={language}
      queryClient={client}
      router={(routes) => <MemoryRouter initialEntries={[url]}>{routes}</MemoryRouter>}
    />,
  ).replaceAll(" ", " ");
}

test("the landing page reads in each language", async () => {
  // Through `prerender`, which waits for the lazy page (#896); `renderToString` does not.
  const landing = (language: Language) => {
    const client = createQueryClient();
    client.setQueryData(ME, null);
    return prerenderHtml(
      <App
        language={language}
        queryClient={client}
        router={(routes) => <MemoryRouter initialEntries={["/login"]}>{routes}</MemoryRouter>}
      />,
    );
  };
  expect(await landing("en")).toContain("It gets made.");
  expect(await landing("pt-BR")).toContain("Ele fica pronto.");
  expect(await landing("es")).toContain("Queda hecho.");
});

test("the shell and the library switch language together, and money stays in dollars", () => {
  const pt = render("/library", "pt-BR", true);
  expect(pt).toContain("Projetos");
  expect(pt).toContain("Biblioteca");
  expect(pt).toContain('aria-label="Configurações"');
  expect(pt).toContain("$1,234.56");
  expect(pt).not.toContain("R$");
  expect(pt).not.toContain(">Projects<");

  const es = render("/library", "es", true);
  expect(es).toContain("Proyectos");
  expect(es).toContain('aria-label="Ajustes"');
  expect(es).toContain("$1,234.56");
});
