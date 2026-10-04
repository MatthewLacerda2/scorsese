// A lista de projetos, os arquivos de um projeto e o histórico de gastos, em
// português do Brasil.

import type { Messages } from "@/i18n/catalogue";

export const pages: Messages["pages"] = {
  projectFiles: {
    noSuchProject: "Esse projeto não existe.",
    back: "Voltar para os projetos",
    openEditor: "Abrir no editor",
    intro: "Os arquivos da sua biblioteca que este projeto usa. Para enviar novos, vá para a",
    libraryLink: "biblioteca",
    introEnd: ".",
  },
  projects: {
    empty: "Nenhum projeto ainda. Dê um nome a um aí em cima para começar.",
    newName: "Nome do novo projeto",
    newPlaceholder: "Nome do novo projeto",
    create: "Criar",
    name: "Nome do projeto",
    changed: (date: string) => `Alterado em ${date}`,
    open: "Abrir",
    files: "Arquivos",
    delete: "Excluir",
    deleteLabel: (name: string) => `Excluir ${name}`,
    deleteTitle: (name: string) => `Excluir “${name}”?`,
    deleteDescription:
      "O projeto é excluído de vez. Os arquivos dele continuam na sua biblioteca, e o que ele custou continua no seu histórico de gastos.",
  },
  spending: {
    balance: "Saldo",
    entries: (count: string, n: number) =>
      `${count} ${n === 1 ? "lançamento" : "lançamentos"}, somando`,
    empty: "Nada mexeu no seu saldo ainda.",
    showMore: "Mostrar mais",
    project: "Projeto",
    allProjects: "Todos os projetos",
    kind: "Tipo",
    everything: "Tudo",
    from: "De (UTC)",
    to: "Até (UTC)",
    clear: "Limpar filtros",
  },
  history: {
    kinds: {
      veo_shot: "Geração de vídeo",
      spoken_line: "Geração de fala",
      still_image: "Geração de imagem",
      voice_design: "Criação de voz",
      assistant: "Assistente",
      top_up: "Recarga",
      monthly_fee: "Mensalidade",
      refund: "Reembolso",
    },
    status: {
      charged: "Cobrado",
      free: "Grátis: o provedor falhou",
      pending: "Pendente",
      credited: "Creditado",
    },
    when: "Quando",
    what: "O quê",
    amount: "Valor",
    balanceAfter: "Saldo depois",
    deletedProject: "um projeto excluído",
    fileItMade: "o arquivo gerado",
  },
};
