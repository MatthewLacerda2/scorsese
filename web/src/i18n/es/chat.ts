// El panel del asistente en el editor, en español. Lo que escribe el
// asistente y lo que responde el servidor se quedan como llegan.

import type { Messages } from "@/i18n/catalogue";

export const chat: Messages["chat"] = {
  loading: "Cargando…",
  jobs: {
    veo_shot: "Toma de video",
    still_image: "Imagen",
    spoken_line: "Locución",
    voice_design: "Diseño de voz",
  },
  jobState: {
    waiting: "en cola",
    running: "generando",
    done: "listo",
    failed: (error) => (error ? `falló: ${error}` : "falló"),
    stuck: (error) =>
      error ? `atascado esperando al proveedor: ${error}` : "atascado esperando al proveedor",
    cancelled: "cancelado",
  },
  ending: {
    refused: "El asistente rechazó esta petición.",
    capped:
      "Detenido: este turno llegó a lo máximo que puede gastar un turno, o se acabó tu saldo.",
    stopped: "Detenido, como pediste.",
    failed: "Este turno falló.",
    interrupted: "El servidor se reinició mientras este turno se ejecutaba.",
  },
  problem: {
    unconfigured: "El asistente aún no está configurado en este servidor.",
    noCredit: "No te quedan créditos para el asistente.",
    busy: "El asistente todavía está trabajando en el último mensaje.",
  },
  composer: {
    placeholder: "Pregúntale al asistente… (Enter envía, Shift+Enter para una nueva línea)",
    answerPlaceholder:
      "Responde la pregunta del asistente: elige una opción de arriba o escribe tu propia respuesta aquí",
    fresh: "Nueva conversación",
    stop: "Detener",
    send: "Enviar",
  },
  turn: {
    busy: [
      "Empalmando…",
      "Retocando el color…",
      "Rebuscando en el metraje…",
      "Alineando los cortes…",
      "Rebobinando la cinta…",
      "Sincronizando el sonido…",
      "Recortando unos fotogramas…",
      "Revisando la toma…",
      "Ajustando el foco…",
      "Preparando el plano…",
      "Poniendo la música…",
      "Buscando el mejor ángulo…",
    ],
    costSoFar: (cost) => `Hasta ahora este turno ha costado ${cost}; nada más mientras espera`,
    cost: (cost) => `Este turno costó ${cost}`,
    left: (balance) => ` · quedan ${balance}`,
  },
  effort: {
    label: "Con cuánto cuidado trabaja el asistente en este mensaje",
    name: { low: "Rápido", medium: "Equilibrado", high: "A fondo" },
    hint: {
      low: "Para retoques pequeños, como agrandar el título o mover la música. Lo más barato.",
      medium: "Para pedidos de todos los días.",
      high: "Para montar o rehacer una edición. Tarda más y cuesta más.",
    },
  },
  model: {
    label: "Modelo",
    unavailable: "no disponible",
    switchTitle: "¿Cambiar de modelo?",
    switchWarning:
      "Cambiar de modelo invalida la caché en el siguiente mensaje. La caché guarda tu " +
      "conversación para que salga más barata. ¿Seguro que quieres cambiar?",
    switch: "Cambiar",
    cost: (words) => `Costo: ${words}`,
  },
  cost: {
    cheapest: "el más barato",
    inexpensive: "barato",
    moderate: "precio medio",
    expensive: "caro",
    dearest: "el más caro",
  },
  quote: {
    ask: (cost) => `Esto cuesta ${cost} de tus créditos. ¿Seguimos?`,
    choose: (now, batch) =>
      `Ahora: ${now} · en menos de 24 horas: ${batch}, mitad de precio en las imágenes. ¿Cuál prefieres?`,
    now: (cost) => `Ahora — ${cost}`,
    batch: (cost) => `En menos de 24 horas — ${cost}`,
    expired: "Este presupuesto ha caducado.",
    confirm: "Confirmar",
    decline: "Rechazar",
    changePlaceholder: "O di qué cambiar; p. ej., haz la capa amarilla en vez de roja",
    change: "Pedir un cambio",
    brief: { prompt: "Prompt", line: "Locución", voice: "Voz" },
    less: "menos",
    more: "más",
  },
  question: {
    notAnswered: "Sin respuesta.",
    ownPlaceholder: "O responde con tus palabras",
    answer: "Responder",
  },
  picker: {
    see: (count: number) => `Ver las ${count} opciones`,
    hint: "Haz clic en una imagen para verla más grande, marca las que te gusten y confirma.",
    ownPlaceholder: "O di lo que prefieres — por ejemplo, algo más oscuro",
    enlarge: "Ver más grande",
    select: "Elegir",
    selected: "Elegida",
    back: "Todas las opciones",
    by: (author: string) => `de ${author}`,
    from: (source: string) => `De ${source}`,
    none: "Ninguna de estas",
    noneSay: "Ninguna — decir esto",
    use: (count: number) => (count > 1 ? `Usar estas ${count}` : "Usar esta"),
    noneTaken: "Ninguna elegida.",
  },
};
