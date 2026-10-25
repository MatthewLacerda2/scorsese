// La política de privacidad en español: una traducción de `pt-BR/privacy.ts`,
// el texto vinculante (`document.ts`), con las mismas secciones y bloques en
// el mismo orden.

import type { LegalDocument } from "@/legal/document";
import { SELLER } from "@/legal/seller";

export const PRIVACY_ES: LegalDocument = {
  title: "Política de privacidad",
  updatedLabel: "Última actualización",
  updated: "2026-10-25",
  summary: {
    heading: "En resumen",
    points: [
      "Entras con Google y guardamos solo tu nombre, tu correo y el identificador de tu cuenta de Google, nunca una contraseña.",
      "Tus archivos y videos son tuyos: están en nuestro servidor en Brasil, separados de los de otras personas.",
      "Para generar lo que pides, cada proveedor recibe solo lo necesario (la lista está abajo).",
      "No vendemos datos, no mostramos anuncios y no usamos tu contenido para entrenar inteligencia artificial.",
      "Puedes pedir una copia o la eliminación de todo por correo, y respondemos en un máximo de 15 días.",
    ],
  },
  sections: [
    {
      heading: "Quién cuida tus datos",
      blocks: [
        `El responsable de tus datos personales, según la Ley General de Protección de Datos de Brasil (LGPD, Ley 13.709/2018), es ${SELLER.name}, CNPJ ${SELLER.cnpj}, ${SELLER.address}. El canal para cualquier asunto de privacidad, incluidas tus solicitudes como titular, es ${SELLER.email}.`,
      ],
    },
    {
      heading: "Qué guardamos",
      blocks: [
        [
          "Cuenta: tu correo, tu nombre y el identificador de tu cuenta de Google, que recibimos cuando entras con Google. Tu foto de Google no se guarda.",
          "Contenido: los archivos que subes (que pueden mostrar personas, incluidos su rostro y su voz), tus proyectos, lo que se genera para ti y los textos que escribes para pedir cada generación.",
          "Conversaciones con el asistente: los mensajes y el registro de cada acción que el asistente hace en tu proyecto. Usamos ese registro para encontrar errores y mejorar el asistente.",
          "Créditos y pagos: tu saldo, el historial de gastos y, de cada compra, el monto, la cotización usada y el identificador del pago en Stripe. Los datos de tu tarjeta van directo a Stripe y nunca pasan por nuestro servidor.",
          "Registros de acceso: la dirección IP, la fecha y la hora de cada vez que entras con Google y del primer ingreso de cada día, guardados por al menos 6 meses, como exige el Marco Civil de Internet de Brasil (Ley 12.965/2014).",
          "Registros del servidor: por cada solicitud al sitio, la dirección IP, la ubicación aproximada que Cloudflare deduce de ella (ciudad, región y país), el navegador (user agent), la dirección solicitada y la fecha y la hora, usados para mantener el servicio seguro y encontrar errores. Nunca registran tu nombre ni tu correo electrónico. Se guardan en una rotación limitada por tamaño, en la que los más antiguos se borran a medida que llegan nuevos.",
        ],
      ],
    },
    {
      heading: "Para qué lo usamos",
      blocks: [
        [
          "Para prestar el servicio que contrataste: guardar y editar tus proyectos, generar lo que pides y cobrar por ello (ejecución de contrato, art. 7, V, de la LGPD).",
          "Para cumplir obligaciones legales, como las fiscales y la conservación de los registros de acceso (art. 7, II).",
          "Para mantener el servicio seguro y mejorar el asistente a partir del registro de las conversaciones (interés legítimo, art. 7, IX).",
        ],
        "No vendemos tus datos, no mostramos anuncios y no usamos tu contenido para entrenar modelos de inteligencia artificial.",
      ],
    },
    {
      heading: "Con quién los compartimos",
      blocks: [
        "Cada proveedor recibe solo lo que necesita para hacer su parte:",
        [
          {
            text: "Anthropic (el asistente Claude): tus mensajes y el contenido del proyecto que el asistente lee para responder.",
            link: {
              label: "Política de privacidad",
              href: "https://www.anthropic.com/legal/privacy",
            },
          },
          {
            text: "Google: los pedidos de video (Veo) y de imagen (Gemini), con las imágenes de referencia que indiques; y el inicio de sesión con Google.",
            link: { label: "Política de privacidad", href: "https://policies.google.com/privacy" },
          },
          {
            text: "ElevenLabs: los textos que se van a narrar y las descripciones de voz.",
            link: { label: "Política de privacidad", href: "https://elevenlabs.io/privacy-policy" },
          },
          {
            text: "Stripe: el pago. Stripe recibe tu correo y los datos de la tarjeta que escribes en su página.",
            link: { label: "Política de privacidad", href: "https://stripe.com/privacy" },
          },
          {
            text: "Cloudflare: todo acceso al sitio pasa por ella, que protege y entrega las páginas.",
            link: {
              label: "Política de privacidad",
              href: "https://www.cloudflare.com/privacypolicy/",
            },
          },
          {
            text: "Pixabay: solo los términos de búsqueda, cuando el asistente busca videos o fotos de archivo.",
            link: { label: "Política de privacidad", href: "https://pixabay.com/service/privacy/" },
          },
          {
            text: "LottieFiles: solo los términos de búsqueda, cuando el asistente busca animaciones de archivo.",
            link: {
              label: "Política de privacidad",
              href: "https://lottiefiles.com/page/privacy-policy",
            },
          },
        ],
        "La mayoría de estos proveedores están fuera de Brasil, principalmente en Estados Unidos. La transferencia se hace para ejecutar el servicio que contrataste (art. 33, IX, de la LGPD) y sigue las reglas de protección de datos de cada uno.",
        "Fuera de esto, solo compartimos datos si una ley o una orden judicial lo exige.",
      ],
    },
    {
      heading: "Dónde se guardan y por cuánto tiempo",
      blocks: [
        "Tus datos están en nuestro servidor, en Brasil, y en una copia de seguridad que se rehace periódicamente y se conserva por 30 días.",
        "Se guardan mientras tu cuenta exista. Cuando la cuenta se cierra, todo se borra del servidor, y las copias de seguridad que aún tengan tus datos vencen en un máximo de 30 días. Los registros de pago siguen en Stripe y en nuestra contabilidad por el plazo que exige la ley fiscal, y los registros de acceso siguen guardados, como exige el Marco Civil. Los registros del servidor no se borran uno a uno: siguen hasta que la rotación los borre.",
      ],
    },
    {
      heading: "Tus derechos",
      blocks: [
        `Según la LGPD (art. 18), puedes pedir en cualquier momento, en ${SELLER.email}: la confirmación de que tratamos tus datos y el acceso a ellos; la corrección de lo que esté mal; una copia de tus datos; la lista de con quién los compartimos; y la eliminación de tus datos y de tu cuenta. Respondemos en un máximo de 15 días.`,
        "También puedes presentar una queja ante la Autoridad Nacional de Protección de Datos de Brasil (ANPD).",
      ],
    },
    {
      heading: "Cookies",
      blocks: [
        "Usamos solo las cookies necesarias para que el sitio funcione: una que te mantiene conectado hasta por 30 días y otra, temporal, que se usa durante el inicio de sesión con Google. No usamos cookies de publicidad ni de análisis. El idioma y el tema que eliges se guardan en tu propio navegador.",
      ],
    },
    {
      heading: "Seguridad",
      blocks: [
        "El acceso al sitio siempre va cifrado, los archivos de cada persona se guardan separados de los de las demás, y lo que el asistente ejecuta a partir de tu contenido corre aislado y sin acceso a internet. Ningún sistema es totalmente seguro: si ocurre un incidente que pueda ponerte en riesgo, te avisaremos a ti y a la ANPD, como manda la ley.",
      ],
    },
    {
      heading: "Menores de edad",
      blocks: ["scorsese es para mayores de 18 años y no recopila, a propósito, datos de menores."],
    },
    {
      heading: "Si no vives en Brasil",
      blocks: [
        "scorsese se opera desde Brasil y sigue la LGPD. Los derechos descritos arriba valen para cualquier persona, donde quiera que viva, por el mismo correo. Si vives en la Unión Europea o en el Reino Unido, también puedes presentar una queja ante la autoridad de protección de datos de tu país. No vendemos ni compartimos datos personales para publicidad, en el sentido de leyes como la de California (CCPA).",
      ],
    },
    {
      heading: "Cambios en esta política",
      blocks: [
        "Si esta política cambia de forma relevante, avisaremos por correo o en el sitio antes de que el cambio entre en vigor. La fecha de arriba indica cuándo se modificó por última vez.",
      ],
    },
  ],
};
