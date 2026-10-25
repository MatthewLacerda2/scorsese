// The privacy policy in English: a translation of `pt-BR/privacy.ts`, the
// binding text (`document.ts`), with the same sections and blocks in the same
// order.

import type { LegalDocument } from "@/legal/document";
import { SELLER } from "@/legal/seller";

export const PRIVACY_EN: LegalDocument = {
  title: "Privacy policy",
  updatedLabel: "Last updated",
  updated: "2026-10-25",
  summary: {
    heading: "In short",
    points: [
      "You sign in with Google and we keep only your name, your email and your Google account identifier, never a password.",
      "Your files and videos are yours: they sit on our server in Brazil, kept apart from other people's.",
      "To generate what you ask for, each supplier receives only what it needs (the list is below).",
      "We do not sell data, we show no ads and we do not use your content to train artificial intelligence.",
      "You can ask for a copy or the deletion of everything by email, and we answer within 15 days.",
    ],
  },
  sections: [
    {
      heading: "Who looks after your data",
      blocks: [
        `The controller of your personal data, under Brazil's General Data Protection Law (LGPD, Law 13.709/2018), is ${SELLER.name}, CNPJ ${SELLER.cnpj}, ${SELLER.address}. The channel for anything about privacy, including your requests as a data subject, is ${SELLER.email}.`,
      ],
    },
    {
      heading: "What we keep",
      blocks: [
        [
          "Account: your email, your name and your Google account identifier, received when you sign in with Google. Your Google photo is not kept.",
          "Content: the files you upload (which may show people, including their faces and voices), your projects, what is generated for you and the texts you write to request each generation.",
          "Conversations with the assistant: the messages and the record of each action the assistant takes on your project. We use this record to find errors and improve the assistant.",
          "Credits and payments: your balance, your spending history and, for each purchase, the amount, the exchange rate used and the payment identifier at Stripe. Your card details go straight to Stripe and never pass through our server.",
          "Access logs: the IP address, date and time of each time you sign in with Google and of your first visit each day, kept for at least 6 months, as Brazil's Marco Civil da Internet (Internet Civil Framework, Law 12.965/2014) requires.",
          ""Server logs: for each request to the site, the IP address, the approximate location Cloudflare works out from it (city, region and country), the browser (user agent), the address requested and the date and time, used to keep the service secure and to find errors. They never record your name or email. They are kept in a rotation limited by size, in which the oldest are deleted as new ones arrive.",
        ],
      ],
    },
    {
      heading: "What we use it for",
      blocks: [
        [
          "To provide the service you signed up for: store and edit your projects, generate what you ask for and charge for it (performance of a contract, LGPD art. 7, V).",
          "To meet legal obligations, such as tax rules and keeping the access logs (art. 7, II).",
          "To keep the service secure and improve the assistant from the conversation record (legitimate interest, art. 7, IX).",
        ],
        "We do not sell your data, we show no ads and we do not use your content to train artificial intelligence models.",
      ],
    },
    {
      heading: "Who we share it with",
      blocks: [
        "Each supplier receives only what it needs to do its part:",
        [
          {
            text: "Anthropic (the Claude assistant): your messages and the project content the assistant reads in order to answer.",
            link: { label: "Privacy policy", href: "https://www.anthropic.com/legal/privacy" },
          },
          {
            text: "Google: video (Veo) and image (Gemini) requests, with the reference images you point to; and signing in with Google.",
            link: { label: "Privacy policy", href: "https://policies.google.com/privacy" },
          },
          {
            text: "ElevenLabs: the texts to be narrated and the voice descriptions.",
            link: { label: "Privacy policy", href: "https://elevenlabs.io/privacy-policy" },
          },
          {
            text: "Stripe: the payment. Stripe receives your email and the card details you type on its page.",
            link: { label: "Privacy policy", href: "https://stripe.com/privacy" },
          },
          {
            text: "Cloudflare: all access to the site passes through it, and it protects and delivers the pages.",
            link: { label: "Privacy policy", href: "https://www.cloudflare.com/privacypolicy/" },
          },
          {
            text: "Pixabay: only the search terms, when the assistant looks for stock videos or photos.",
            link: { label: "Privacy policy", href: "https://pixabay.com/service/privacy/" },
          },
          {
            text: "LottieFiles: only the search terms, when the assistant looks for stock animations.",
            link: { label: "Privacy policy", href: "https://lottiefiles.com/page/privacy-policy" },
          },
        ],
        "Most of these suppliers are outside Brazil, mainly in the United States. The transfer is made to carry out the service you signed up for (LGPD art. 33, IX) and follows each supplier's own data protection rules.",
        "Beyond this, we share data only if a law or a court order requires it.",
      ],
    },
    {
      heading: "Where it is kept and for how long",
      blocks: [
        "Your data is kept on our server, in Brazil, and in a backup that is redone periodically and kept for 30 days.",
        "It is kept as long as your account exists. When the account is closed, everything is deleted from the server, and backups that still hold your data expire within 30 days. Payment records remain at Stripe and in our accounting for as long as tax law requires, and the access logs remain kept, as the Marco Civil requires.",
      ],
    },
    {
      heading: "Your rights",
      blocks: [
        `Under the LGPD (art. 18), you may ask at any time, at ${SELLER.email}, for: confirmation that we process your data and access to it; correction of anything wrong; a copy of your data; the list of who we share it with; and deletion of your data and your account. We answer within 15 days.`,
        "You may also complain to Brazil's National Data Protection Authority (ANPD).",
      ],
    },
    {
      heading: "Cookies",
      blocks: [
        "We use only the cookies the site needs to work: one that keeps you signed in for up to 30 days and another, temporary one, used while you sign in with Google. We use no advertising or analytics cookies. The language and theme you choose are kept in your own browser.",
      ],
    },
    {
      heading: "Security",
      blocks: [
        "Access to the site is always encrypted, each person's files are kept apart from everyone else's, and whatever the assistant runs from your content runs isolated and without internet access. No system is completely secure: if an incident happens that could put you at risk, we will tell you and the ANPD, as the law requires.",
      ],
    },
    {
      heading: "Minors",
      blocks: ["scorsese is for people over 18 and does not knowingly collect data from minors."],
    },
    {
      heading: "If you do not live in Brazil",
      blocks: [
        "scorsese is operated from Brazil and follows the LGPD. The rights described above apply to anyone, wherever they live, through the same email. If you live in the European Union or the United Kingdom, you may also complain to your country's data protection authority. We do not sell or share personal data for advertising, in the sense of laws such as California's (CCPA).",
      ],
    },
    {
      heading: "Changes to this policy",
      blocks: [
        "If this policy changes in a significant way, we will tell you by email or on the site before the change takes effect. The date at the top says when it was last changed.",
      ],
    },
  ],
};
