// El panel del asistente en el editor, en español. Lo que escribe el
// asistente y lo que responde el servidor se quedan como llegan.

import type { Messages } from "@/i18n/catalogue";

export const chat: Messages["chat"] = {
  loading: "Cargando…",
  /** The one line shown while any generation is running; the dots animate. */
  generating: "Generando",
  ending: {
    refused: "El asistente rechazó esta petición.",
    capped:
      "Detenido: este turno llegó a lo máximo que puede gastar un turno, o se acabó tu saldo.",
    stopped: "Detenido, como pediste.",
    failed:
      "Este turno falló. Si sigue fallando, empieza una conversación nueva: continúa con el proyecto tal como está.",
    interrupted: "El servidor se reinició mientras este turno se ejecutaba.",
  },
  problem: {
    unconfigured: "El asistente aún no está configurado en este servidor.",
    noCredit: "No te quedan créditos para el asistente.",
    busy: "El asistente todavía está trabajando en el último mensaje.",
    missing: "Estos archivos no llegaron con el inicio de sesión. Adjúntalos de nuevo y envía:",
  },
  full: {
    enter: "Expandir",
    leave: "Volver al editor",
  },
  script: {
    open: "Guion",
    hint: "El guion a partir del cual el asistente monta tu video",
    title: "Guion",
    description:
      "El asistente monta el video a partir de él. Cambia lo que quieras: leerá tu versión la próxima vez.",
    empty: "Todavía no hay guion — el asistente escribe uno cuando empiezas un video.",
    edit: "Editar",
    write: "Escribir uno",
    save: "Guardar",
    cancel: "Cancelar",
    moved:
      "El asistente cambió el guion mientras editabas, así que el tuyo no se guardó. Aquí está como quedó, con tu versión debajo para copiar.",
    yours: "Tu versión, sin guardar",
    changed:
      "El asistente acaba de cambiar el guion. Guardar no lo sobrescribe: tu versión queda a su lado para copiar.",
    conflict: "El proyecto está cambiando ahora mismo; intenta guardar de nuevo en un momento.",
    close: "Cerrar",
    failed: "No se pudo guardar el guion.",
  },
  composer: {
    placeholder: "Pregúntale al asistente… (Enter envía, Shift+Enter para una nueva línea)",
    answerPlaceholder:
      "Responde la pregunta del asistente: elige una opción de arriba o escribe tu propia respuesta aquí",
    fresh: "Nueva conversación",
    freshTitle: "¿Empezar una nueva conversación? Esta queda guardada.",
    freshStart: "Empezar",
    freshCancel: "Cancelar",
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
    woken: "Siguió cuando tus generaciones estuvieron listas",
    left: (balance) => ` · quedan ${balance}`,
  },
  effort: {
    label: "Cuánto piensa el asistente",
    name: { low: "Bajo", medium: "Medio", high: "Alto", xhigh: "Extra" },
    hint: {
      low: "Para retoques pequeños, como agrandar el título o mover la música. Lo más barato.",
      medium: "Para pedidos de todos los días.",
      high: "Para montar o rehacer una edición. Tarda más y cuesta más.",
      xhigh:
        "Para los trabajos más grandes, como un video entero a partir de un brief largo. Lo más lento y lo más caro.",
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
    costLabel: "Costo",
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
    answered: {
      confirmed: (cost) => `Aprobado — ${cost}`,
      declined: (cost) => `Rechazado — ${cost}`,
      changed: (cost) => `Cambio pedido — ${cost}`,
      withdrawn: (cost) => `Sin respuesta — ${cost}`,
    },
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
