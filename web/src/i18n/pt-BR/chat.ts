// O painel do assistente no editor, em português do Brasil. O que o
// assistente escreve e o que o servidor responde ficam como vieram.

import type { Messages } from "@/i18n/catalogue";

export const chat: Messages["chat"] = {
  loading: "Carregando…",
  jobs: {
    veo_shot: "Cena em vídeo",
    still_image: "Imagem",
    spoken_line: "Fala",
    voice_design: "Criação de voz",
  },
  jobState: {
    waiting: "na fila",
    running: "gerando",
    done: "pronto",
    failed: (error) => (error ? `falhou: ${error}` : "falhou"),
    stuck: (error) =>
      error ? `travado esperando o provedor: ${error}` : "travado esperando o provedor",
    cancelled: "cancelado",
  },
  ending: {
    refused: "O assistente recusou este pedido.",
    capped: "Parado: este turno chegou ao limite de gastos de um turno, ou seus créditos acabaram.",
    stopped: "Parado, como você pediu.",
    failed: "Este turno falhou.",
    interrupted: "O servidor reiniciou enquanto este turno rodava.",
  },
  problem: {
    unconfigured: "O assistente ainda não está configurado neste servidor.",
    noCredit: "Você não tem mais créditos para o assistente.",
    busy: "O assistente ainda está trabalhando na última mensagem.",
  },
  composer: {
    placeholder: "Pergunte ao assistente… (Enter envia, Shift+Enter pula linha)",
    answerPlaceholder:
      "Responda à pergunta do assistente — escolha uma opção acima ou escreva sua resposta aqui",
    fresh: "Nova conversa",
    stop: "Parar",
    send: "Enviar",
  },
  turn: {
    working: "Trabalhando…",
    costSoFar: (cost) => `Até agora este turno custou ${cost}; nada mais enquanto espera`,
    cost: (cost) => `Este turno custou ${cost}`,
    left: (balance) => ` · restam ${balance}`,
  },
  model: {
    label: "Modelo",
    unavailable: "indisponível",
    switchTitle: "Trocar de modelo?",
    switchWarning:
      "Trocar de modelo invalida o cache no próximo prompt. O cache guarda sua conversa para " +
      "deixá-la mais barata. Tem certeza de que quer trocar?",
    switch: "Trocar",
    cost: (words) => `Custo: ${words}`,
  },
  cost: {
    cheapest: "o mais barato",
    inexpensive: "barato",
    moderate: "preço médio",
    expensive: "caro",
    dearest: "o mais caro",
  },
  quote: {
    ask: (cost) => `Isso custa ${cost} dos seus créditos. Pode seguir?`,
    expired: "Este orçamento expirou.",
    confirm: "Confirmar",
    decline: "Recusar",
    changePlaceholder: "Ou diga o que mudar — ex.: deixe a capa amarela em vez de vermelha",
    change: "Pedir uma mudança",
    brief: { prompt: "Prompt", line: "Fala", voice: "Voz" },
    less: "menos",
    more: "mais",
  },
  question: {
    notAnswered: "Sem resposta.",
    ownPlaceholder: "Ou responda com suas palavras",
    answer: "Responder",
  },
};
