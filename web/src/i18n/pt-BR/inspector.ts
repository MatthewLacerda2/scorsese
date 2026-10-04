// O inspetor — os valores de um clipe, a duração e o loop de uma sequência de
// imagens, e o briefing de um clipe gerado.

import type { Messages } from "@/i18n/catalogue";

export const inspector: Messages["inspector"] = {
  notInAssets: "não está na lista de recursos",
  onTrack: (track: string) => `na faixa ${track}`,
  start: "Início",
  duration: "Duração",
  speed: "Velocidade",
  fit: "Ajuste",
  fits: {
    fit: "Encaixar — imagem inteira",
    fill: "Preencher — sem barras",
    native: "Original — tamanho em pixels próprio",
  },
  position: "Posição",
  positionTitle: "Quanto ela sai do lugar natural: para a direita e para baixo, em % do quadro",
  rotation: "Rotação",
  scale: "Escala",
  animated: "Animado",
  points: (n: number) => (n === 1 ? "1 ponto" : `${n} pontos`),
  byHand: "à mão",
  animationNote: "Para mudar uma animação, é só pedir ao assistente.",
  animatedValue: "animado",
  animatedTitle: "Isto muda ao longo do clipe — nada aqui vai deixar fixo",
  stretched: (wide: number, tall: number) => `${wide}% de largura, ${tall}% de altura`,
  stretchedTitle: "Esticado de propósito — peça ao assistente um valor único",
  sequence: {
    title: "Sequência",
    count: (stills: number, hold: number, frames: number) =>
      `${stills} ${stills === 1 ? "imagem" : "imagens"} × ${hold} ${hold === 1 ? "quadro" : "quadros"} = ${frames} quadros`,
    hold: "Duração",
    holdTitle: "Quantos quadros cada imagem fica na tela",
    loop: "Repetir",
    loopTitle: "Recomeçar da primeira imagem, em vez de parar na última",
  },
  brief: {
    title: "Briefing",
    noneYet: "nada ainda",
    note: "Para mudar o briefing, ou criar um, é só pedir ao assistente.",
    labels: { prompt: "Prompt", line: "Fala", recipe: "Receita" },
    choices: {
      model: "Modelo",
      size: "Tamanho",
      length: "Duração",
      aspect: "Proporção",
      voice: "Voz",
      language: "Idioma",
    },
    states: {
      sketch: "ainda não foi feito — aparece na prévia como cartela, sem custo",
      queued: "sendo feito",
      generated: "feito",
      stale:
        "o briefing mudou depois que foi feito — aparece na prévia como cartela até ser feito de novo",
    },
  },
};
