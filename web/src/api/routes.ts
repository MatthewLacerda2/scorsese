// The parsing behind routes.test.ts: docs/web.md's route rows, and the routes
// src/api names, both as `{method, path}` with every parameter written `{}`.
// Kept apart from the test, which pins these with cases of its own.

/** A route: `method` is null for a URL the browser follows, by any method. */
export interface Route {
  method: string | null;
  path: string;
}

const METHOD = "GET|POST|PUT|PATCH|DELETE";

/** Every `` | `METHOD /api/…` | `` row, its `{id}`s written `{}`. */
export function documented(markdown: string): Route[] {
  const row = new RegExp(`^\\| \`(${METHOD}) (/api/[^\`\\s]*)\` \\|`, "gm");
  return [...markdown.matchAll(row)].map(([, method, path]) => ({
    method: method ?? null,
    path: (path ?? "").replace(/\{[^}]*\}/g, "{}"),
  }));
}

/**
 * The routes a source file names: each `request("METHOD", path)` (the path
 * under `/api`), and each other literal that starts `/api/`.
 */
export function calls(source: string): Route[] {
  const found: Route[] = [];
  const request = new RegExp(
    `\\brequest(?:<[^()]*?>)?\\(\\s*"(${METHOD})"\\s*,\\s*("[^"]*"|\`[^\`]*\`)`,
    "g",
  );
  const spans: [number, number][] = [];
  for (const match of source.matchAll(request)) {
    const [whole, method, literal] = match;
    found.push({ method: method ?? null, path: `/api${pathOf(literal ?? "")}` });
    spans.push([match.index, match.index + whole.length]);
  }
  for (const match of source.matchAll(/(["`])(\/api\/[^"`]*)\1/g)) {
    if (spans.some(([from, to]) => match.index >= from && match.index < to)) continue;
    found.push({ method: null, path: pathOf(`${match[1]}${match[2]}${match[1]}`) });
  }
  return found;
}

/** A literal's path: quotes off, `${…}` as `{}`, `${query(…)}` and `?…` dropped. */
export function pathOf(literal: string): string {
  const body = literal.slice(1, -1);
  let path = "";
  for (let at = 0; at < body.length; ) {
    if (!body.startsWith("${", at)) {
      path += body[at];
      at += 1;
      continue;
    }
    let depth = 1;
    let end = at + 2;
    for (; end < body.length && depth > 0; end += 1) {
      if (body[end] === "{") depth += 1;
      if (body[end] === "}") depth -= 1;
    }
    const expression = body.slice(at + 2, end - 1).trim();
    if (!expression.startsWith("query(")) path += "{}";
    at = end;
  }
  return path.split("?")[0] ?? path;
}

/** Whether `call` is `row`: the same path, and the same method unless the browser picks it. */
export function matches(row: Route, call: Route): boolean {
  return row.path === call.path && (call.method === null || row.method === call.method);
}
