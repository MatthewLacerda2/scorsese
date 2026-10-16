// Nothing here is language a user reads: symbols, numbers, single letters,
// acronyms, the product's name, words from a call, and literals JSX computes
// with rather than shows.
declare const t: { files: { title: string; count: (n: number) => string } };
declare const name: string;
declare const busy: boolean;
declare function cn(...classes: string[]): string;

export const symbols = <p>— · % ↓ … 3/4 1080p</p>;
export const letter = <kbd>K</kbd>;
export const acronym = <span>MP4 · HD · OK</span>;
export const product = <a href="/">scorsese</a>;
export const fromCatalogue = <p title={t.files.title}>{t.files.count(3)}</p>;
export const classes = <p className={cn("text-sm", busy ? "opacity-50" : "font-bold")} />;
export const comparison = <p>{name === "draft" ? t.files.title : name}</p>;
export const key = <p>{{ draft: 1 }[name]}</p>;
export const notText = <div aria-hidden="true" role="separator" data-state="open" />;
export const interpolated = <p>{`${busy ? "saving" : name}`}</p>;
export const spaced = (
  <p>
    {name}
  </p>
);
