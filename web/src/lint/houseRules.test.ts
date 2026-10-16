// The house rules (web/README.md, *House rules*) held to their fixtures: every
// rule in lint/ is loaded by both configs, a `flags*` fixture is reported on
// exactly its lines marked `// flagged`, and a `passes*` fixture on none.
// It lives under src/ because that is where `bun test` looks (bunfig.toml).
import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const WEB = join(import.meta.dir, "..", "..");
const LINT = join(WEB, "lint");
const FIXTURES = join(LINT, "fixtures");
const RULES = readdirSync(LINT)
  .filter((name) => name.endsWith(".grit"))
  .map((name) => name.slice(0, -".grit".length));

type Diagnostic = { message: string; location: { path: string; start: { line: number } } };

function lintFixtures(): Diagnostic[] {
  const run = Bun.spawnSync(
    [
      join(WEB, "node_modules", ".bin", "biome"),
      "lint",
      "--config-path",
      join(FIXTURES, "rules.jsonc"),
      "--reporter=json",
      "--max-diagnostics=none",
      FIXTURES,
    ],
    { cwd: WEB },
  );
  return JSON.parse(run.stdout.toString()).diagnostics;
}

const diagnostics = lintFixtures();

function flaggedLines(rule: string, file: string): number[] {
  const lines = diagnostics
    .filter((d) => d.location.path.endsWith(`${rule}/${file}`))
    .map((d) => {
      expect(d.message).toStartWith(`${rule}:`);
      return d.location.start.line;
    });
  return [...new Set(lines)].sort((a, b) => a - b);
}

test("there is at least one house rule", () => {
  expect(RULES.length).toBeGreaterThan(0);
});

for (const rule of RULES) {
  describe(rule, () => {
    test("is loaded by the web's config and the fixtures' config", () => {
      expect(readFileSync(join(WEB, "biome.jsonc"), "utf8")).toContain(`"./lint/${rule}.grit"`);
      expect(readFileSync(join(FIXTURES, "rules.jsonc"), "utf8")).toContain(`"../${rule}.grit"`);
    });

    const files = readdirSync(join(FIXTURES, rule));
    test("has a fixture it must flag and one it must pass", () => {
      expect(files.some((f) => f.startsWith("flags"))).toBe(true);
      expect(files.some((f) => f.startsWith("passes"))).toBe(true);
    });

    for (const file of files) {
      test(file, () => {
        const marked = readFileSync(join(FIXTURES, rule, file), "utf8")
          .split("\n")
          .flatMap((line, i) => (line.endsWith("// flagged") ? [i + 1] : []));
        if (file.startsWith("passes")) expect(marked).toEqual([]);
        expect(flaggedLines(rule, file)).toEqual(marked);
      });
    }
  });
}
