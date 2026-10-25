// Comprar créditos (#548), em português do Brasil.

import type { Messages } from "@/i18n/catalogue";

export const buying: Messages["buying"] = {
  empty: "Você ainda não tem créditos.",
  add: "Adicionar créditos",
  description:
    "Você paga em reais e os créditos ficam em dólares. O pagamento é feito pelo Stripe; o scorsese nunca vê o seu cartão.",
  accepted: "Pix ou cartão de crédito Visa/Mastercard.",
  amount: "Valor em reais",
  preview: (reais, dollars) => `${reais} → ${dollars} em créditos`,
  bonus: (dollars, percent) => `+ ${dollars} de bônus (+${percent}%)`,
  rate: (rate) => `A R$ ${rate.replace(".", ",")} por dólar, travado no momento em que você paga.`,
  limits: (least, most) => `De ${least} a ${most} por vez.`,
  pay: "Pagar",
  paying: "Abrindo o Stripe…",
  returned: {
    title: "Seu pagamento",
    checking: "Conferindo com o Stripe…",
    paid: (dollars) => `Pago. ${dollars} foram adicionados ao seu saldo.`,
    open: "O Stripe ainda não confirmou o pagamento. Esta página se atualiza sozinha.",
    expired: "Este pagamento expirou antes de ser pago. Nada foi cobrado.",
    missing: "Esse pagamento não existe.",
    projects: "Voltar aos projetos",
    history: "Ver o histórico de gastos",
  },
};
