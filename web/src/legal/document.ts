// A legal document as data: a plain summary and numbered sections of
// paragraphs and lists, drawn by `LegalPage`. There is one document per
// language the site speaks (`pt-BR/`, `en/`, `es/`), chosen by the site's
// language. The Portuguese text is the contract: the other two are
// translations, and the page says so in the reader's language and offers the
// Portuguese text beside them. `documents.test.ts` holds the three to the same
// structure, so a translation cannot gain or lose a section, a paragraph or a
// list item unseen.

/** A list item: words, or words followed by a link (a vendor and its own policy). */
export type Item = string | { text: string; link: { label: string; href: string } };

/** A paragraph, or a list when it is an array. */
export type Block = string | readonly Item[];

interface Section {
  heading: string;
  blocks: readonly Block[];
}

export interface LegalDocument {
  title: string;
  /** The label before the date, "Última atualização". */
  updatedLabel: string;
  /** When the text last changed, `YYYY-MM-DD`. */
  updated: string;
  /** The plain summary above section 1: its heading and a few short points. */
  summary: { heading: string; points: readonly string[] };
  sections: readonly Section[];
}
