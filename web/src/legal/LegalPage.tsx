// A legal document as a page (#547): `/terms` and `/privacy`, public and
// crawlable, in the landing page's dark art direction since that is where a
// visitor follows the link from. The document is Portuguese whatever the
// site's language (`document.ts` has why), so its own text is marked
// `lang="pt-BR"` and a visitor reading in another language is told why.

import "@/landing/landing.css";
import { useEffect } from "react";
import { Link } from "react-router";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import type { LegalDocument } from "@/legal/document";
import { Footer } from "@/legal/Footer";

/** `2026-10-14` as "14 de outubro de 2026". */
function longDate(day: string): string {
  return new Intl.DateTimeFormat("pt-BR", { dateStyle: "long", timeZone: "UTC" }).format(
    new Date(`${day}T00:00:00Z`),
  );
}

export function LegalPage({ document: doc }: { document: LegalDocument }) {
  const t = useT();
  const { language } = useLanguage();
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
        <Link to="/" className="display text-xl text-[#f4efe6]">
          scorsese
        </Link>
        {language !== "pt-BR" && (
          <p className="dim mt-8 rounded-lg border border-[#f4efe6]/15 p-4 text-sm">
            {t.landing.legal.portugueseOnly}
          </p>
        )}
        <article lang="pt-BR" className="mt-10">
          <h1 className="display text-4xl sm:text-5xl">{doc.title}</h1>
          <p className="dim mt-3 text-sm">Última atualização: {longDate(doc.updated)}</p>
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
                  <ul key={block[0]} className="dim mt-4 list-disc space-y-2 pl-6 leading-relaxed">
                    {block.map((item) => (
                      <li key={item}>{item}</li>
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
