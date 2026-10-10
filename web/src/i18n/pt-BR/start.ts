// Começar um projeto (#1016): a janela de novo projeto e o controle de
// plataforma e estilo do editor, em português do Brasil.

import type { Messages } from "@/i18n/catalogue";

export const start: Messages["start"] = {
  title: "Novo projeto",
  description:
    "Dê um nome e, se já souber, diga para onde ele vai. Tudo além do nome pode ficar para depois.",
  steps: {
    name: "Nome",
    assets: "Arquivos",
    platform: "Plataforma",
    style: "Estilo",
  },
  stepOf: (step: number, total: number) => `Passo ${step} de ${total}`,
  name: "Nome do projeto",
  nameMissing: "Escreva o nome do projeto",
  assetsHint:
    "Escolha arquivos da sua biblioteca para trazer ao projeto. Dá para adicionar mais depois.",
  picked: (n: number) => (n === 1 ? "1 arquivo escolhido" : `${n} arquivos escolhidos`),
  platformHint: "Você pode escolher ou mudar de plataforma depois.",
  styleHint: "Você pode escolher ou mudar de estilo depois.",
  noPlatform: "Sem plataforma",
  noStyle: "Sem estilo",
  everyStyle: "Todos os estilos aparecem: escolha uma plataforma para ver os feitos para ela.",
  noPreview: "Prévia em breve",
  back: "Voltar",
  next: "Próximo",
  create: "Criar",
  button: "Plataforma e estilo",
  changeTitle: "Plataforma e estilo",
  changeDescription:
    "O assistente atualiza o roteiro e conta o que muda. Nada na edição muda até você responder.",
  save: "Salvar",
  changedMessage: "Mudei a plataforma ou o estilo deste vídeo.",
  notTold: (why: string) => `Salvo, mas não deu para avisar o assistente: ${why}`,
};
