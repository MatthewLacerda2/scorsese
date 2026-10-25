// The pricing page at /pricing: what is free, what a purchase gives, what
// things cost and the questions people ask. Dollars arrive formatted
// (`lib/money.ts`); reais are written "R$ 20".

export const pricing = {
  meta: {
    title: "Pricing · scorsese",
    description:
      "Free credit every day, and credits from R$ 20 that never expire. You see the price and confirm before every paid generation.",
  },
  title: "Pay for what you make",
  lead: "Start free, every day. Buy credits only when you want more, and see the price before anything is spent.",
  free: {
    name: "Free",
    price: (daily: string) => `${daily} a day`,
    lines: (monthly: string) => [
      "In credit, renewed every day",
      `Up to ${monthly} a month`,
      "No card needed",
    ],
    button: "Start free",
  },
  plan: {
    name: "Credits",
    reais: (amount: number) => `R$ ${amount}`,
    bonus: (percent: number) => `+${percent}% bonus credit`,
    best: "Best value",
    lines: ["Converted into US dollars at the day's rate when you buy", "Never expire"],
    button: "Buy",
  },
  costs: {
    title: "What things cost",
    lead: "Real examples, at what you pay.",
    image: "An image",
    narration: "A line of narration",
    music: "40 seconds of music",
    video: "8 seconds of generated video, at the default quality",
    assistant: "A message to the assistant, depending on the work",
    around: (amount: string) => `≈ ${amount}`,
    footnote:
      "A 1-minute narrated video with images and music usually fits in a day's free credit.",
  },
  faq: {
    title: "Questions",
    termsLink: "Read the terms",
    items: [
      {
        q: "Will I know the price before I pay?",
        a: "Yes. Before every paid generation scorsese shows what it will cost, and generates only after you confirm.",
      },
      {
        q: "What if a generation fails?",
        a: "A generation that fails is not charged, and the credits go back to your balance.",
      },
      {
        q: "Do credits expire?",
        a: "Never. What you buy stays in your balance until you use it.",
      },
      {
        q: "Can I cancel a purchase?",
        a: "Within 7 days of buying, you can withdraw and get back what you have not spent.",
      },
      {
        q: "How do I pay?",
        a: "In reais, by Pix or card, through Stripe. Your card details go straight to Stripe and never reach us.",
      },
    ],
  },
};
