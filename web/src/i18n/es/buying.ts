// Comprar créditos (#548), en español.

import type { Messages } from "@/i18n/catalogue";

export const buying: Messages["buying"] = {
  empty: "Aún no tienes créditos.",
  add: "Añadir créditos",
  description:
    "Pagas en reales y tus créditos quedan en dólares. Stripe procesa el pago; scorsese nunca ve tu tarjeta.",
  accepted: "Pix o tarjeta de crédito Visa/Mastercard.",
  amount: "Importe en reales",
  preview: (reais, dollars) => `${reais} → ${dollars} en créditos`,
  bonus: (dollars, percent) => `+ ${dollars} de bono (+${percent}%)`,
  rate: (rate) => `A R$ ${rate.replace(".", ",")} por dólar, fijado en el momento en que pagas.`,
  limits: (least, most) => `Entre ${least} y ${most} cada vez.`,
  pay: "Pagar",
  paying: "Abriendo Stripe…",
  returned: {
    title: "Tu pago",
    checking: "Comprobando con Stripe…",
    paid: (dollars) => `Pagado. Se añadieron ${dollars} a tu saldo.`,
    open: "Stripe aún no ha confirmado el pago. Esta página se actualiza sola.",
    expired: "Este pago caducó antes de pagarse. No se cobró nada.",
    missing: "Ese pago no existe.",
    projects: "Volver a los proyectos",
    history: "Ver el historial de gastos",
  },
};
