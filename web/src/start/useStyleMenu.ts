// The menu of platforms and styles a project is started for (#1016), from
// `GET /api/styles`, which both pickers choose from.

import { useQuery } from "@tanstack/react-query";
import { useCallback } from "react";
import { api, type StyleMenu } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useT } from "@/i18n/I18nProvider";
import { localized } from "./menu";

/**
 * The menu, read once — it is compiled into the server and never changes
 * under a page — and shown in the page's language.
 */
export function useStyleMenu() {
  const words = useT().menu;
  const select = useCallback((menu: StyleMenu) => localized(menu, words), [words]);
  return useQuery({
    queryKey: queryKeys.styles,
    queryFn: api.styles.menu,
    staleTime: Infinity,
    select,
  });
}
