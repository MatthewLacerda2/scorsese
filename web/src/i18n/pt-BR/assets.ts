// A barra lateral de recursos do editor e o modal da biblioteca, em português
// do Brasil. IDs de recursos e nomes de arquivos são do usuário e ficam como estão.

import type { Messages } from "@/i18n/catalogue";

export const assets: Messages["assets"] = {
  heading: "Recursos",
  empty: "Nada ainda. Adicione arquivos da sua biblioteca e arraste-os para uma faixa.",
  alsoAlone: "também usada sozinha",
  fold: (id) => `Recolher as fotos de ${id}`,
  unfold: (id) => `Mostrar as fotos de ${id}`,
  drag: "Arraste para uma faixa para posicionar",
  photos: (name, n) => `${name} — ${n === 1 ? "1 foto" : `${n} fotos`}`,
  remove: (id) => `Remover ${id}`,
  removeTitle: "Remover do projeto — os clipes que o usam também saem",
  state: { sketch: "rascunho", queued: "na fila", generated: "gerado", stale: "desatualizado" },
  library: {
    button: "Biblioteca",
    title: "Adicionar da sua biblioteca",
    description:
      "Escolha um arquivo para adicioná-lo aos recursos deste projeto e depois arraste-o de lá " +
      "para uma faixa. Você pode enviar arquivos novos aqui ou soltá-los nesta janela.",
    inProject: "já está no projeto",
  },
  kinds: {
    video: "vídeo",
    image: "imagem",
    audio: "áudio",
    text: "texto",
    color: "cor",
    shape: "forma",
    icon: "ícone",
    group: "grupo",
    image_sequence: "sequência de imagens",
    html: "página",
    generated_video: "vídeo gerado",
    generated_image: "imagem gerada",
    generated_audio: "fala gerada",
    synth_audio: "áudio sintetizado",
  },
};
