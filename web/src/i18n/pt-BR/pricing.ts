import type { Messages } from "@/i18n/catalogue";

export const pricing: Messages["pricing"] = {
  meta: {
    title: "Preços · scorsese",
    description:
      "Crédito grátis todo dia, e créditos a partir de R$ 20 que nunca expiram. Você vê o preço e confirma antes de cada geração paga.",
  },
  title: "Pague pelo que você cria",
  lead: "Comece de graça, todo dia. Compre créditos só quando quiser mais, e veja o preço antes de gastar qualquer coisa.",
  free: {
    name: "Grátis",
    price: (daily) => `${daily} por dia`,
    lines: (monthly) => [
      "Em crédito, renovado todo dia",
      `Até ${monthly} por mês`,
      "Sem precisar de cartão",
    ],
    button: "Começar de graça",
  },
  plan: {
    name: "Créditos",
    reais: (amount) => `R$ ${amount}`,
    bonus: (percent) => `+${percent}% de crédito de bônus`,
    best: "Melhor custo-benefício",
    lines: ["Convertidos em dólares americanos pela cotação do dia da compra", "Nunca expiram"],
    button: "Comprar",
  },
  costs: {
    title: "Quanto as coisas custam",
    lead: "Exemplos reais, pelo que você paga.",
    image: "Uma imagem",
    narration: "Uma frase de narração",
    music: "40 segundos de música",
    video: "8 segundos de vídeo gerado, na qualidade padrão",
    assistant: "Uma mensagem ao assistente, conforme o trabalho",
    around: (amount) => `≈ ${amount}`,
    footnote:
      "Um vídeo narrado de 1 minuto, com imagens e música, normalmente cabe no crédito grátis de um dia.",
  },
  faq: {
    title: "Perguntas",
    termsLink: "Ler os termos",
    items: [
      {
        q: "Vou saber o preço antes de pagar?",
        a: "Sim. Antes de cada geração paga, o scorsese mostra quanto ela vai custar e só gera depois que você confirma.",
      },
      {
        q: "E se uma geração falhar?",
        a: "Geração que falha não é cobrada, e os créditos voltam para o seu saldo.",
      },
      {
        q: "Os créditos expiram?",
        a: "Nunca. O que você compra fica no seu saldo até você usar.",
      },
      {
        q: "Posso desistir de uma compra?",
        a: "Em até 7 dias depois de comprar, você pode desistir e receber de volta o que não gastou.",
      },
      {
        q: "Como eu pago?",
        a: "Em reais, por Pix ou cartão, pela Stripe. Os dados do seu cartão vão direto para a Stripe e nunca chegam até nós.",
      },
    ],
  },
};
