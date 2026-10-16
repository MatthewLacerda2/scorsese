// Buying credits (#548), in English — the source the other catalogues are
// typed against. Reais and dollars arrive already formatted; the server's own
// sentences (a refusal, a rate that could not be read) stay as they come.

export const buying = {
  empty: "You have no credits yet.",
  add: "Add credits",
  description:
    "You pay in reais and your credits are in dollars. Stripe takes the payment; scorsese never sees your card.",
  accepted: "Pix or Visa/Mastercard credit card.",
  amount: "Amount in reais",
  // "R$ 50,00 → $10.02 in credits"
  preview: (reais: string, dollars: string) => `${reais} → ${dollars} in credits`,
  rate: (rate: string) => `At R$ ${rate} to the dollar, kept from the moment you pay.`,
  limits: (least: string, most: string) => `Between ${least} and ${most} at a time.`,
  pay: "Pay",
  paying: "Opening Stripe…",
  returned: {
    title: "Your payment",
    checking: "Checking with Stripe…",
    paid: (dollars: string) => `Paid. ${dollars} were added to your balance.`,
    open: "Stripe has not confirmed the payment yet. This page updates on its own.",
    expired: "This payment expired before it was paid. Nothing was charged.",
    missing: "There is no such payment.",
    projects: "Back to projects",
    history: "See your spending history",
  },
};
