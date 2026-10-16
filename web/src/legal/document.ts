// A legal document as data: numbered sections of paragraphs and lists, drawn
// by `LegalPage`. The text is in Portuguese only, whatever language the site
// is read in: it is the language of the contract (the seller and its first
// customers are Brazilian), and a translation would be a second text that
// could disagree with the first. The page says so to a visitor reading in
// another language.

/** A paragraph, or a list of items when it is an array. */
type Block = string | readonly string[];

interface Section {
  heading: string;
  blocks: readonly Block[];
}

export interface LegalDocument {
  title: string;
  /** When the text last changed, `YYYY-MM-DD`. */
  updated: string;
  sections: readonly Section[];
}
