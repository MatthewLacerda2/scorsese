// A public page's title and description, in the visitor's language, for as
// long as the page is shown. `index.html` holds the English ones a link
// preview reads (it runs no script); a crawler that does run it, and the
// browser's tab, get these.

import { useEffect } from "react";

export function useMeta(title: string, description?: string) {
  useEffect(() => {
    const before = document.title;
    document.title = title;
    const meta = document.querySelector<HTMLMetaElement>('meta[name="description"]');
    const was = meta?.content;
    if (meta && description) meta.content = description;
    return () => {
      document.title = before;
      if (meta && was !== undefined) meta.content = was;
    };
  }, [title, description]);
}
