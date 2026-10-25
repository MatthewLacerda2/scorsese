import type { Messages } from "@/i18n/catalogue";

export const pricing: Messages["pricing"] = {
  meta: {
    title: "Precios · scorsese",
    description:
      "Crédito gratis cada día, y créditos desde R$ 20 que no vencen. Ves el precio y confirmas antes de cada generación de pago.",
  },
  title: "Paga por lo que creas",
  lead: "Empieza gratis, cada día. Compra créditos solo cuando quieras más, y ve el precio antes de gastar nada.",
  free: {
    name: "Gratis",
    price: (daily) => `${daily} al día`,
    lines: (monthly) => [
      "En crédito, que se renueva cada día",
      `Hasta ${monthly} al mes`,
      "Sin necesidad de tarjeta",
    ],
    button: "Empezar gratis",
  },
  plan: {
    name: "Créditos",
    reais: (amount) => `R$ ${amount}`,
    bonus: (percent) => `+${percent}% de crédito de bonificación`,
    credits: (dollars) => `≈ ${dollars} en créditos`,
    best: "La mejor opción",
    lines: [
      "Convertidos a dólares estadounidenses a la cotización del día de la compra",
      "No vencen",
    ],
    button: "Comprar",
    buying: "Abriendo Stripe…",
  },
  costs: {
    title: "Cuánto cuestan las cosas",
    lead: "Ejemplos reales, a lo que tú pagas.",
    image: "Una imagen",
    narration: "Una línea de narración",
    music: "40 segundos de música",
    video: "8 segundos de video generado, en la calidad predeterminada",
    around: (amount) => `≈ ${amount}`,
    footnote:
      "Un video narrado de 1 minuto, con imágenes y música, normalmente cabe en el crédito gratis de un día.",
  },
  faq: {
    title: "Preguntas",
    termsLink: "Leer los términos",
    items: [
      {
        q: "¿Sabré el precio antes de pagar?",
        a: "Sí. Antes de cada generación de pago, scorsese muestra cuánto va a costar y solo genera después de que confirmas.",
      },
      {
        q: "¿Y si una generación falla?",
        a: "Una generación que falla no se cobra, y los créditos vuelven a tu saldo.",
      },
      {
        q: "¿Los créditos vencen?",
        a: "Nunca. Lo que compras queda en tu saldo hasta que lo uses.",
      },
      {
        q: "¿Puedo desistir de una compra?",
        a: "Dentro de los 7 días siguientes a la compra, puedes desistir y recibir de vuelta lo que no gastaste.",
      },
      {
        q: "¿Cómo pago?",
        a: "En reales, con Pix o tarjeta, a través de Stripe. Los datos de tu tarjeta van directo a Stripe y nunca nos llegan.",
      },
    ],
  },
};
