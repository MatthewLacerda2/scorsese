// A legal document as a page: `/terms` and `/privacy`, public and crawlable,
// in the landing page's look (and its light or dark) since that is where a
// visitor follows the link from. It is drawn in the site's language; the
// Portuguese text is the binding one (`document.ts`), so a visitor reading a
// translation is told so, and can switch to the Portuguese text and back.
// The article's `lang` is the language of the text it holds.

import "@/landing/landing.css";
import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";
import type { Item, LegalDocument } from "@/legal/document";
import { Footer } from "@/legal/Footer";

/** `2026-10-14` as "14 de outubro de 2026", in the document's language. */
function longDate(day: string, language: Language): string {
  return new Intl.DateTimeFormat(language, { dateStyle: "long", timeZone: "UTC" }).format(
    new Date(`${day}T00:00:00Z`),
  );
}

function ListItem({ item }: { item: Item }) {
  if (typeof item === "string") return <li>{item}</li>;
  return (
    <li>
      {item.text}{" "}
      <a
        href={item.link.href}
        target="_blank"
        rel="noopener noreferrer"
        className="text-paper underline underline-offset-2"
      >
        {item.link.label}
      </a>
    </li>
  );
}

export function LegalPage({
  documents,
  language,
}: {
  documents: Record<Language, LegalDocument>;
  language: Language;
}) {
  const t = useT().landing.legal;
  const [binding, setBinding] = useState(false);
  const shown: Language = binding ? "pt-BR" : language;
  const doc = documents[shown];
  const translated = language !== "pt-BR";
  useEffect(() => {
    const before = document.title;
    document.title = `${doc.title} · scorsese`;
    return () => {
      document.title = before;
    };
  }, [doc.title]);

  return (
    <div className="landing">
      <main className="mx-auto max-w-3xl px-4 py-12 sm:px-6 sm:py-16">
        <Link to="/" className="display text-xl text-paper">
          scorsese
        </Link>
        {translated && (
          <p className="dim mt-8 rounded-lg border border-paper/15 p-4 text-sm">
            {t.translation}{" "}
            <Button
              type="button"
              variant="link"
              className="h-auto p-0 text-paper"
              onClick={() => setBinding(!binding)}
            >
              {binding ? t.readTranslation : t.readPortuguese}
            </Button>
          </p>
        )}
        <article lang={shown} className="mt-10">
          <h1 className="display text-4xl sm:text-5xl">{doc.title}</h1>
          <p className="dim mt-3 text-sm">
            {doc.updatedLabel}: {longDate(doc.updated, shown)}
          </p>
          <aside className="card mt-8 p-5 sm:p-6">
            <h2 className="text-lg font-semibold">{doc.summary.heading}</h2>
            <ul className="dim mt-3 list-disc space-y-2 pl-5 leading-relaxed">
              {doc.summary.points.map((point) => (
                <li key={point}>{point}</li>
              ))}
            </ul>
          </aside>
          {doc.sections.map((section, index) => (
            <section key={section.heading} className="mt-10">
              <h2 className="text-xl font-semibold">
                {index + 1}. {section.heading}
              </h2>
              {section.blocks.map((block) =>
                typeof block === "string" ? (
                  <p key={block} className="dim mt-4 leading-relaxed">
                    {block}
                  </p>
                ) : (
                  <ul
                    key={typeof block[0] === "string" ? block[0] : block[0]?.text}
                    className="dim mt-4 list-disc space-y-2 pl-6 leading-relaxed"
                  >
                    {block.map((item) => (
                      <ListItem key={typeof item === "string" ? item : item.text} item={item} />
                    ))}
                  </ul>
                ),
              )}
            </section>
          ))}
        </article>
      </main>
      <Footer />
    </div>
  );
}
