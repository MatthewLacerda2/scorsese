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
    busy: [
      "Emendando…",
      "Acertando as cores…",
      "Vasculhando as imagens…",
      "Alinhando os cortes…",
      "Rebobinando a fita…",
      "Sincronizando o som…",
      "Aparando uns quadros…",
      "Conferindo a tomada…",
      "Ajustando o foco…",
      "Montando a cena…",
      "Soltando a trilha…",
      "Procurando o melhor ângulo…",
    ],
    costSoFar: (cost) => `Até agora este turno custou ${cost}; nada mais enquanto espera`,
    cost: (cost) => `Este turno custou ${cost}`,
    left: (balance) => ` · restam ${balance}`,
  },
  effort: {
    label: "Com quanto cuidado o assistente trabalha nesta mensagem",
    name: { low: "Rápido", medium: "Equilibrado", high: "Caprichado" },
    hint: {
      low: "Para ajustes pequenos, como aumentar o título ou mover a música. O mais barato.",
      medium: "Para pedidos do dia a dia.",
      high: "Para montar ou refazer uma edição. Demora mais e custa mais.",
    },
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
    choose: (now, batch) =>
      `Agora: ${now} · em até 24 horas: ${batch}, metade do preço nas imagens. Qual prefere?`,
    now: (cost) => `Agora — ${cost}`,
    batch: (cost) => `Em até 24 horas — ${cost}`,
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
  picker: {
    see: (count: number) => `Ver as ${count} opções`,
    hint: "Clique numa imagem para vê-la maior, marque as que gostar e confirme.",
    ownPlaceholder: "Ou diga o que prefere — por exemplo, algo mais escuro",
    enlarge: "Ver maior",
    select: "Escolher",
    selected: "Escolhida",
    back: "Todas as opções",
    by: (author: string) => `por ${author}`,
    from: (source: string) => `Do ${source}`,
    none: "Nenhuma destas",
    noneSay: "Nenhuma — dizer isto",
    use: (count: number) => (count > 1 ? `Usar estas ${count}` : "Usar esta"),
    noneTaken: "Nenhuma escolhida.",
  },
};
