// O navegador de arquivos da biblioteca, os detalhes de um arquivo e a bandeja
// de envios, em português do Brasil.

import type { Messages } from "@/i18n/catalogue";

export const files: Messages["files"] = {
  kinds: { video: "Vídeo", image: "Imagem", audio: "Áudio", midi: "MIDI" },
  sorts: {
    newest: "Mais recentes",
    oldest: "Mais antigos",
    name: "Nome",
    largest: "Maiores primeiro",
  },
  browser: {
    label: "Arquivos",
    search: "Buscar pelo nome",
    kind: "Tipo",
    allKinds: "Todos os tipos",
    sort: "Ordenar",
    upload: "Enviar",
    noMatch: "Nenhum arquivo encontrado.",
    emptyLibrary:
      "Sua biblioteca está vazia. Envie vídeos, imagens, sons e arquivos MIDI — ou arraste para cá — e use em qualquer projeto.",
    emptyProject: "Este projeto ainda não usa nenhum arquivo da sua biblioteca.",
    add: (name: string) => `Adicionar ${name}`,
  },
  edit: (label: string) => `Editar ${label.toLowerCase()}`,
  viewer: {
    midi: "Um arquivo MIDI guarda notas, não som. Peça ao assistente para fazer a trilha do seu vídeo com ele.",
  },
  download: {
    button: "Baixar",
    file: (name: string) => `Baixar ${name}`,
  },
  details: {
    notFound: "Não encontrado",
    name: "Nome",
    generated: "gerado",
    view: "Ver",
    open: "Abrir",
    play: "Reproduzir",
    size: "Tamanho",
    dimensions: "Dimensões",
    duration: "Duração",
    added: "Adicionado em",
    description: "Descrição",
    descriptionPlaceholder: "O que o assistente lê na hora de escolher um arquivo",
    usedIn: "Usado em",
    unused: "Nenhum projeto usa ainda.",
    templates: (names: string) => `Modelos: ${names}`,
    delete: "Excluir",
    deleteTitle: (name: string) => `Excluir “${name}”?`,
    deleteDescription: "O arquivo sai da sua biblioteca de vez.",
  },
  generation: {
    title: {
      veo_shot: "Vídeo gerado",
      still_image: "Imagem gerada",
      spoken_line: "Fala gerada",
      voice_design: "Amostra de criação de voz",
    },
    model: "Modelo",
    made: "Feito em",
    cost: "Custo",
    free: "Nada foi cobrado",
    estimate: "Estimativa",
    listedPrice: (amount: string) => `${amount} pelo preço de tabela do provedor`,
    resolution: "Resolução",
    length: "Duração",
    seconds: (seconds: string) => `${seconds} s`,
    aspect: "Proporção",
    references: "Referências",
    seed: "Semente",
    voice: "Voz",
  },
  uploads: {
    title: "Envios",
    clear: "Limpar concluídos",
    phase: {
      hashing: "Verificando…",
      uploading: "Enviando…",
      done: "Enviado",
      duplicate: "Já está na sua biblioteca",
      failed: "Falhou",
    },
    alreadyHave: (name: string) => `você já tem este arquivo como “${name}”`,
  },
};
