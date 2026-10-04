// The chosen language, given to every component under it.
//
// `useT()` is the messages in the page's language; `useLanguage()` is the
// language itself (which is also the locale `Intl` formats numbers and dates
// in) and a way to change it. Outside a provider — a component test rendering
// one piece on its own — both read English, so those tests need no setup.

import { createContext, type ReactNode, useCallback, useContext, useEffect, useState } from "react";
import { type Messages, messagesFor } from "@/i18n/catalogue";
import { en } from "@/i18n/en";
import { browserStorage, initialLanguage, type Language, saveLanguage } from "@/i18n/language";

interface I18n {
  language: Language;
  messages: Messages;
  choose: (language: Language) => void;
}

const Context = createContext<I18n>({ language: "en", messages: en, choose: () => {} });

interface Props {
  children: ReactNode;
  /** Start in this language instead of the browser's — for tests. */
  initial?: Language;
}

export function I18nProvider({ children, initial }: Props) {
  const [language, setLanguage] = useState<Language>(() => initial ?? initialLanguage());
  const [messages, setMessages] = useState<Messages>(() => messagesFor(language));

  // Screen readers and the browser's own spell-check and hyphenation read it.
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const choose = useCallback((next: Language) => {
    saveLanguage(browserStorage(), next);
    setLanguage(next);
    setMessages(messagesFor(next));
  }, []);

  return <Context value={{ language, messages, choose }}>{children}</Context>;
}

/** Every user-visible string, in the page's language. */
export function useT(): Messages {
  return useContext(Context).messages;
}

/** The page's language (the `Intl` locale too), and a way to change it. */
export function useLanguage(): { language: Language; choose: (language: Language) => void } {
  const { language, choose } = useContext(Context);
  return { language, choose };
}
