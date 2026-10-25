// The pricing page's questions, each one opening on a click. A native
// `<details>`, so the keyboard and screen readers already know it.

import { Link } from "react-router";
import { useT } from "@/i18n/I18nProvider";

/** The question about withdrawing a purchase, whose answer points to the terms. */
const WITHDRAW = 3;

export function Faq() {
  const t = useT().pricing.faq;
  return (
    <section className="mt-20">
      <h2 className="display text-3xl font-medium sm:text-4xl">{t.title}</h2>
      <div className="mt-8 divide-y divide-paper/10 border-y border-paper/10">
        {t.items.map((item, index) => (
          <details key={item.q} className="group py-4">
            <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-lg font-medium">
              {item.q}
              <span className="dim transition-transform group-open:rotate-45" aria-hidden>
                +
              </span>
            </summary>
            <p className="dim mt-3 leading-relaxed">
              {item.a}{" "}
              {index === WITHDRAW && (
                <Link to="/terms" className="text-paper underline underline-offset-2">
                  {t.termsLink}
                </Link>
              )}
            </p>
          </details>
        ))}
      </div>
    </section>
  );
}
