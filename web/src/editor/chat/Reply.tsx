// The assistant's words, as Markdown (#768). Both of the picker's vendors
// write it by habit, so printed plain it shows its markers; rendered, a plan
// reads as a list and a choice stands out in bold.
//
// The text is the model's, so it is untrusted, and this is safe by
// construction: `react-markdown` builds React elements and never sets HTML, so
// a `<script>` or an `<img onerror>` in the text arrives as the text it is
// (there is no `rehype-raw` here, and there must never be). Its default URL
// filter turns a `javascript:` link into an empty one. An image is never
// fetched — a model-written `![](…)` would reach an arbitrary URL from the
// user's browser — so it shows as its alt text instead. Links open in a new
// tab, away from the editor.
//
// While a turn streams, the text is re-rendered whole on every piece. Markdown
// has no unclosed markers to fail on — a lone `**` is two asterisks and an open
// fence runs to the end — so half a reply is always a valid document.

import type { ReactNode } from "react";
import Markdown, { type Components } from "react-markdown";

/** Each element in the chat's small type: no page-sized heading, no page-wide code block. */
const COMPONENTS: Components = {
  h1: ({ children }) => <Heading>{children}</Heading>,
  h2: ({ children }) => <Heading>{children}</Heading>,
  h3: ({ children }) => <Heading>{children}</Heading>,
  h4: ({ children }) => <Heading>{children}</Heading>,
  h5: ({ children }) => <Heading>{children}</Heading>,
  h6: ({ children }) => <Heading>{children}</Heading>,
  p: ({ children }) => <p className="whitespace-pre-wrap">{children}</p>,
  ul: ({ children }) => <ul className="list-disc space-y-0.5 pl-5">{children}</ul>,
  ol: ({ children }) => <ol className="list-decimal space-y-0.5 pl-5">{children}</ol>,
  blockquote: ({ children }) => (
    <blockquote className="border-l-2 pl-2 text-muted-foreground">{children}</blockquote>
  ),
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="underline underline-offset-2"
    >
      {children}
    </a>
  ),
  pre: ({ children }) => (
    <pre className="overflow-x-auto rounded-md bg-muted p-2 text-xs [&_code]:bg-transparent [&_code]:p-0">
      {children}
    </pre>
  ),
  code: ({ children }) => (
    <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">{children}</code>
  ),
  img: ({ alt }) => <span>{alt}</span>,
  hr: () => <hr className="border-border" />,
};

function Heading({ children }: { children?: ReactNode }) {
  return <p className="font-semibold">{children}</p>;
}

/** What the assistant said, rendered. */
export function Reply({ text }: { text: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-2 text-sm break-words">
      <Markdown components={COMPONENTS}>{text}</Markdown>
    </div>
  );
}
