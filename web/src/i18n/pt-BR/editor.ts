// O editor — a página, a renderização, as remoções, os modelos, a prévia e a
// linha do tempo. O inspetor fica em `inspector.ts`.

import type { Messages } from "@/i18n/catalogue";

const more = (named: string, rest: number) => `${named} e mais ${rest}`;

export const editor: Messages["editor"] = {
  page: {
    opening: "Abrindo o projeto…",
    frameShape: "Formato do quadro",
    render: "Renderizar",
    renderTitle: "Renderizar o vídeo",
    renderDescription: "O corte inteiro, do jeito que está agora, em MP4 para baixar.",
    dismiss: "Dispensar",
    resizeAssets: "Redimensionar o painel de recursos",
    resizeChat: "Redimensionar o painel do chat",
    resizeTimeline: "Redimensionar a linha do tempo",
  },
  refusal: {
    failed: "não deu para fazer a edição",
    conflict:
      "O projeto mudou enquanto você editava (o assistente ou outra aba), então essa edição não foi feita. Aqui está o que tem agora.",
  },
  render: {
    resolution: "Resolução",
    render: "Renderizar",
    stop: "Parar",
    download: "Baixar",
    audio: "áudio",
    none: "Nenhuma renderização guardada para este projeto ainda.",
    note: "As renderizações ficam guardadas por um tempo e são refeitas quando você pede; não custam créditos.",
    jobs: {
      waiting: "Na fila, atrás de outras renderizações…",
      running: "Renderizando…",
      done: "Pronto — está logo abaixo.",
      failed: "A renderização falhou",
      stuck: "A renderização travou",
      cancelled: "Parada — nada foi guardado",
    },
  },
  progress: {
    preparing: (percent: string) => `${percent} · preparando…`,
    mixing: (percent: string) => `${percent} · mixando o som…`,
    finishing: (percent: string) => `${percent} · finalizando o arquivo…`,
  },
  removal: {
    more,
    asset: (asset: string) => `Remover “${asset}” do projeto?`,
    assetClips: (asset: string, count: number, clips: string) =>
      `Remover “${asset}” do projeto? ${count === 1 ? "Este clipe usa o recurso e também será excluído" : `Estes ${count} clipes usam o recurso e também serão excluídos`}: ${clips}.`,
    track: (track: string) => `Remover a faixa “${track}”?`,
    trackClips: (track: string, count: number, clips: string) =>
      `Remover a faixa “${track}”? ${count === 1 ? "O clipe que está nela também será excluído" : `Os ${count} clipes que estão nela também serão excluídos`}: ${clips}.`,
  },
  states: {
    sketch: "rascunho",
    queued: "na fila",
    generated: "gerado",
    stale: "desatualizado",
  },
  trackKinds: { video: "vídeo", audio: "áudio" },
  timeline: {
    empty: "Arraste algo da direita para cá.",
    newTrack: "Solte aqui para uma nova faixa",
    removeTrack: (track: string) => `Remover a faixa ${track}`,
    removeTrackTitle: "Remover a faixa — os clipes que estão nela vão junto",
    clipTitle: (clip: string) =>
      `${clip} — arraste para mover (inclusive para outra faixa), arraste uma borda para cortar, Shift+clique para selecionar vários, Delete para remover`,
  },
  preview: {
    queued: "Prévia na fila…",
    rendering: "Renderizando a prévia…",
    ready: "Prévia pronta",
    failed: "Não deu para renderizar a prévia.",
    failedWhy: (why: string) => `Não deu para renderizar a prévia: ${why}`,
    empty: "Nada na linha do tempo ainda.",
    frame: "O quadro sob a agulha",
    start: "Ir para o início",
    back: "Voltar um quadro",
    play: "Reproduzir",
    pause: "Pausar",
    forward: "Avançar um quadro",
    end: "Ir para o fim",
    scrub: "Percorrer",
    quality: "Qualidade da prévia",
    qualities: {
      full: { label: "Máxima", says: "qualidade máxima: a imagem da própria renderização" },
      half: { label: "1/2", says: "meia qualidade: menos pixels, com proxies onde houver" },
      quarter: {
        label: "1/4",
        says: "um quarto da qualidade: a mais rápida, com proxies onde houver",
      },
    },
  },
  templates: {
    title: "Modelos",
    none: "Nenhum ainda. Selecione clipes na linha do tempo e salve como modelo.",
    summary: (seconds: string, clips: number) =>
      `${seconds}s · ${clips === 1 ? "1 clipe" : `${clips} clipes`}`,
    insert: (name: string) => `Inserir ${name} na agulha`,
    insertTitle: (seconds: string) => `Inserir na agulha (${seconds}s)`,
    remove: (name: string) => `Excluir ${name}`,
    removeTitle: "Excluir o modelo — os vídeos em que ele foi usado ficam com as cópias deles",
    confirmRemove: (name: string) => `Excluir o modelo “${name}”?`,
    save: "Salvar como modelo",
    saveTitle:
      "Selecione clipes na linha do tempo (Shift+clique para vários) e salve para reutilizar",
    saveDescription: (count: number) =>
      `${count === 1 ? "O clipe selecionado" : `Os ${count} clipes selecionados`}, para usar depois em qualquer projeto seu. Mudar o modelo nunca muda um vídeo em que ele já foi usado.`,
    name: "Nome do modelo",
    placeholder: "Abertura, encerramento, legenda de nome…",
    replace: "Substituir o modelo que tem esse nome",
  },
};
