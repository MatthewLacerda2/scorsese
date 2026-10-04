// A URL no page answers.

import { Link } from "react-router";
import { useT } from "@/i18n/I18nProvider";

export function NotFound() {
  const t = useT();
  return (
    <p className="text-muted-foreground">
      {t.common.notFound.text}{" "}
      <Link to="/projects" className="underline">
        {t.common.notFound.link}
      </Link>
    </p>
  );
}
