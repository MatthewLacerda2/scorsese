// The terms of use in English: a translation of `pt-BR/terms.ts`, the binding
// text (`document.ts`), with the same sections and blocks in the same order.

import type { LegalDocument } from "@/legal/document";
import { SELLER } from "@/legal/seller";

export const TERMS_EN: LegalDocument = {
  title: "Terms of use",
  updatedLabel: "Last updated",
  updated: "2026-10-24",
  summary: {
    heading: "In short",
    points: [
      "No subscription: you buy credits, from R$ 20 to R$ 5,000, and they never expire.",
      "Before every paid generation you see the price and confirm.",
      "A generation that fails is not charged.",
      "You have 7 days to cancel a purchase and get back what you have not spent.",
      "What you upload and make is yours, and how you use it is your responsibility.",
    ],
  },
  sections: [
    {
      heading: "Who we are",
      blocks: [
        `scorsese, at ${SELLER.site}, is offered by ${SELLER.name}, CNPJ ${SELLER.cnpj} (Brazil's company registry number), at ${SELLER.address}. To talk to us about anything, including cancellation, refunds and your data, write to ${SELLER.email}.`,
        "By creating your account or using the site, you agree to these terms and to our Privacy policy. If you do not agree, do not use the service.",
      ],
    },
    {
      heading: "The service",
      blocks: [
        "scorsese is a video editor with an assistant: you upload your files, describe the video you want, and the assistant builds and edits it for you. Part of the work (narration, generated images and videos, and the assistant itself) is done by third-party artificial intelligence services, which we hire and pass on to you.",
        "The scorsese program also exists as free software, which anyone can install and run on their own computer with their own keys. These terms cover only the website, which is the ready-to-use version.",
      ],
    },
    {
      heading: "Your account",
      blocks: [
        "You sign in with your Google account, and your first sign-in creates your scorsese account. You must be 18 or older.",
        "The account is personal. You are responsible for what is done with it and must tell us if you suspect someone is using it without your permission.",
      ],
    },
    {
      heading: "Credits and prices",
      blocks: [
        "There is no subscription. You buy credits and they are spent as you use the service.",
        [
          "Credits are bought in Brazilian reais, from R$ 20 to R$ 5,000 per purchase, through Stripe, by Pix (Brazil's instant payment) or Visa or Mastercard credit card (other brands, such as Elo and American Express, and debit cards are not accepted). The amount is converted to US dollars (US$) at the rate shown when you buy, because the services we use charge us in dollars. Your balance is shown in US$.",
          "Before every generation (narration, image, video and others), scorsese shows how much it will cost and generates only after you confirm.",
          "Each message to the assistant uses credits according to the work it does to answer, with a maximum per message. The cost of each one appears in your spending history.",
          "Credits do not expire.",
          "Prices follow what our suppliers charge us and may change. A change never affects a generation you have already confirmed.",
        ],
      ],
    },
    {
      heading: "Generations that fail and results you do not like",
      blocks: [
        "If a generation fails, through our fault or the supplier's, it is not charged and the credits return to your balance.",
        "A generation that was made but did not turn out to your taste is charged as usual, because the supplier charges us for it all the same. You can check every charge in your spending history.",
      ],
    },
    {
      heading: "Refunds and the right to cancel",
      blocks: [
        `You may cancel a credit purchase within 7 days of making it, as article 49 of Brazil's Consumer Protection Code (CDC) provides. Just write to ${SELLER.email}. We return the part of that purchase that has not yet been spent, by the same payment method.`,
        "After the 7 days, purchased credits are not refunded, but they stay in your balance with no deadline to use them.",
      ],
    },
    {
      heading: "Your content",
      blocks: [
        "The files you upload and the videos you make are yours. You authorise us to store them, process them and send them to suppliers only as far as needed to provide the service, as the Privacy policy describes.",
        "You declare that you have the necessary rights over what you upload, including the image and voice of the people who appear in your files. You may not use scorsese for:",
        [
          "illegal content, or content that sexualises minors in any form;",
          "imitating a real person without their permission, including their voice, or misleading anyone about who speaks or appears in a video;",
          "infringing the copyright, trademark or image rights of others;",
          "harassment, hate speech or incitement to violence;",
          "trying to get around the service's limits, charges or security.",
        ],
        "The artificial intelligence suppliers have their own rules and may refuse a request. We may suspend or close an account that breaks these terms.",
      ],
    },
    {
      heading: "Artificial intelligence results",
      blocks: [
        "What artificial intelligence generates may come out different from what you asked for, or contain mistakes. Review the video before publishing it or using it commercially. You are responsible for how you use it.",
      ],
    },
    {
      heading: "Availability",
      blocks: [
        "scorsese runs on our own server and may be offline for maintenance or because of a fault. We make backups, but we recommend you also keep a copy of the files and videos that matter to you.",
        "To the extent the law allows, our liability for damages related to the service is limited to the amount you paid in the previous 12 months.",
      ],
    },
    {
      heading: "Closing your account",
      blocks: [
        `You may ask us to close your account at any time, at ${SELLER.email}. The account, projects and files are deleted within 15 days. Any remaining balance is lost, except what is still within the cancellation period described above, which is returned.`,
      ],
    },
    {
      heading: "Changes to these terms",
      blocks: [
        "If we change these terms in a significant way, we will tell you by email or on the site at least 15 days ahead. Continuing to use the service after that means agreeing to the new version.",
      ],
    },
    {
      heading: "If you do not live in Brazil",
      blocks: [
        "You may use scorsese from any country. Credits are charged in Brazilian reais, and your bank may convert the amount and charge its own fees. The rights that your country's consumer law guarantees and that a contract cannot waive remain in force.",
      ],
    },
    {
      heading: "Governing law and courts",
      blocks: [
        "These terms follow Brazilian law. The courts of Brasília, Federal District, are chosen, without prejudice to your right, as a consumer, to sue in the courts where you live.",
      ],
    },
  ],
};
