// O painel do assistente no editor, em português do Brasil. O que o
// assistente escreve e o que o servidor responde ficam como vieram.

import type { Messages } from "@/i18n/catalogue";

export const chat: Messages["chat"] = {
  loading: "Carregando…",
  /** The one line shown while any generation is running; the dots animate. */
  generating: "Gerando",
  ending: {
    refused: "O assistente recusou este pedido.",
    capped: "Parado: este turno chegou ao limite de gastos de um turno, ou seus créditos acabaram.",
    stopped: "Parado, como você pediu.",
    failed:
      "Este turno falhou. Se continuar falhando, comece uma conversa nova: ela continua com o projeto como está.",
    interrupted: "O servidor reiniciou enquanto este turno rodava.",
  },
  problem: {
    unconfigured: "O assistente ainda não está configurado neste servidor.",
    noCredit: "Você não tem mais créditos para o assistente.",
    busy: "O assistente ainda está trabalhando na última mensagem.",
    missing: "Estes arquivos não vieram com o login. Anexe-os de novo e envie:",
  },
  full: {
    enter: "Expandir",
    leave: "Voltar ao editor",
  },
  script: {
    open: "Roteiro",
    hint: "O roteiro a partir do qual o assistente monta seu vídeo",
    title: "Roteiro",
    description:
      "É a partir dele que o assistente monta o vídeo. Mude o que quiser: ele lê a sua versão na próxima vez.",
    empty: "Ainda não há roteiro — o assistente escreve um quando você começa um vídeo.",
    edit: "Editar",
    write: "Escrever um",
    save: "Salvar",
    cancel: "Cancelar",
    moved:
      "O assistente mudou o roteiro enquanto você editava, então o seu não foi salvo. Aqui está como ele está agora, com a sua versão abaixo para copiar.",
    yours: "Sua versão, não salva",
    changed:
      "O assistente acabou de mudar o roteiro. Salvar não escreve por cima: a sua versão fica ao lado para copiar.",
    conflict: "O projeto está mudando agora; tente salvar de novo daqui a pouco.",
    close: "Fechar",
    failed: "Não foi possível salvar o roteiro.",
  },
  composer: {
    placeholder: "Pergunte ao assistente… (Enter envia, Shift+Enter pula linha)",
    answerPlaceholder:
      "Responda à pergunta do assistente — escolha uma opção acima ou escreva sua resposta aqui",
    fresh: "Nova conversa",
    freshTitle: "Começar uma nova conversa? Esta continua salva.",
    freshStart: "Começar",
    freshCancel: "Cancelar",
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
    woken: "Continuou quando suas gerações ficaram prontas",
    left: (balance) => ` · restam ${balance}`,
  },
  effort: {
    label: "Quanto o assistente pensa",
    name: { low: "Baixo", medium: "Médio", high: "Alto", xhigh: "Extra" },
    hint: {
      low: "Para ajustes pequenos, como aumentar o título ou mover a música. O mais barato.",
      medium: "Para pedidos do dia a dia.",
      high: "Para montar ou refazer uma edição. Demora mais e custa mais.",
      xhigh:
        "Para os maiores trabalhos, como um vídeo inteiro a partir de um briefing longo. O mais demorado e o mais caro.",
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
    costLabel: "Custo",
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
    answered: {
      confirmed: (cost) => `Aprovado — ${cost}`,
      declined: (cost) => `Recusado — ${cost}`,
      changed: (cost) => `Mudança pedida — ${cost}`,
      withdrawn: (cost) => `Sem resposta — ${cost}`,
    },
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
