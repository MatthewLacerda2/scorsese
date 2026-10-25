// Los términos de uso en español: una traducción de `pt-BR/terms.ts`, el
// texto vinculante (`document.ts`), con las mismas secciones y bloques en el
// mismo orden.

import type { LegalDocument } from "@/legal/document";
import { SELLER } from "@/legal/seller";

export const TERMS_ES: LegalDocument = {
  title: "Términos de uso",
  updatedLabel: "Última actualización",
  updated: "2026-10-25",
  summary: {
    heading: "En resumen",
    points: [
      "Sin mensualidad: compras créditos, de R$ 20 a R$ 5.000, y no vencen.",
      "Toda cuenta tiene US$ 4 gratis al día, hasta US$ 12 al mes, y las compras desde R$ 40 reciben un bono.",
      "Antes de cada generación de pago ves el precio y confirmas.",
      "Una generación que falla no se cobra.",
      "Tienes 7 días para desistir de una compra y recibir de vuelta lo que no gastaste.",
      "Lo que subes y haces es tuyo, y el uso que le des es responsabilidad tuya.",
    ],
  },
  sections: [
    {
      heading: "Quiénes somos",
      blocks: [
        `scorsese, en ${SELLER.site}, lo ofrece ${SELLER.name}, CNPJ ${SELLER.cnpj} (el número de registro de empresas de Brasil), con domicilio en ${SELLER.address}. Para hablar con nosotros de cualquier tema, incluidos la cancelación, el reembolso y tus datos, escribe a ${SELLER.email}.`,
        "Al crear tu cuenta o usar el sitio, aceptas estos términos y nuestra Política de privacidad. Si no estás de acuerdo, no uses el servicio.",
      ],
    },
    {
      heading: "El servicio",
      blocks: [
        "scorsese es un editor de video con un asistente: subes tus archivos, describes el video que quieres, y el asistente lo arma y lo edita por ti. Parte del trabajo (la narración, las imágenes y los videos generados, y el propio asistente) lo hacen servicios de inteligencia artificial de terceros, que nosotros contratamos y te trasladamos.",
        "El programa de scorsese también existe como software libre, que cualquiera puede instalar y usar en su propio computador con sus propias claves. Estos términos tratan solo del sitio web, que es la versión lista para usar.",
      ],
    },
    {
      heading: "Tu cuenta",
      blocks: [
        "Entras con tu cuenta de Google, y el primer ingreso crea tu cuenta de scorsese. Debes tener 18 años o más.",
        "La cuenta es personal. Eres responsable de lo que se haga con ella y debes avisarnos si sospechas que alguien la usa sin tu permiso.",
      ],
    },
    {
      heading: "Créditos y precios",
      blocks: [
        "No hay mensualidad. Compras créditos y se gastan según el uso.",
        [
          "Los créditos se compran en reales brasileños, de R$ 20 a R$ 5.000 por compra, a través de Stripe, con Pix (el pago instantáneo de Brasil) o tarjeta de crédito Visa o Mastercard (no se aceptan otras marcas, como Elo y American Express, ni tarjetas de débito). El monto se convierte a dólares estadounidenses (US$) con la cotización que se muestra al comprar, porque los servicios que usamos nos cobran en dólares. Tu saldo se muestra en US$.",
          "Antes de cada generación (narración, imagen, video y otras), scorsese muestra cuánto va a costar y solo genera después de que confirmas.",
          "Cada mensaje al asistente consume créditos según el trabajo que hace para responder, con un límite máximo por mensaje. El costo de cada uno aparece en tu historial de gastos.",
          "Los créditos no vencen.",
          "Toda cuenta puede gastar hasta US$ 4 al día, y como máximo US$ 12 al mes, sin pagar nada, contados en la hora de Brasilia (el día cambia a medianoche, el mes el día 1). Este uso gratuito va antes del saldo comprado, vale también para quien ya compró y nunca paga un video de Veo. Si una generación pagada con él falla, el monto vuelve al uso gratuito, no al saldo. Lo que no se usa no se acumula, y estos valores pueden cambiar.",
          "Las compras desde R$ 40 reciben un 5% más en créditos de bono, y desde R$ 80, un 10% más. El bono se muestra antes de pagar y entra en el saldo junto con la compra. Estos tramos pueden cambiar; un cambio nunca afecta una compra ya hecha.",
          "Los precios siguen lo que nos cobran los proveedores y pueden cambiar. Un cambio nunca afecta una generación que ya confirmaste.",
        ],
      ],
    },
    {
      heading: "Generaciones que fallan y resultados que no te gustan",
      blocks: [
        "Si una generación falla, por error nuestro o del proveedor, no se cobra y los créditos vuelven a tu saldo.",
        "Una generación que se hizo pero no quedó a tu gusto se cobra con normalidad, porque el proveedor nos la cobra igual. Puedes revisar cada cobro en tu historial de gastos.",
      ],
    },
    {
      heading: "Reembolso y derecho de arrepentimiento",
      blocks: [
        `Puedes desistir de una compra de créditos dentro de los 7 días siguientes a hacerla, como prevé el artículo 49 del Código de Defensa del Consumidor de Brasil (CDC). Basta con escribir a ${SELLER.email}. Te devolvemos la parte de esa compra que aún no se gastó, por el mismo medio de pago, y retiramos de tu saldo su bono que tampoco se gastó, en la misma proporción.`,
        "Pasados los 7 días, los créditos comprados no se reembolsan, pero siguen en tu saldo, sin plazo para usarlos.",
      ],
    },
    {
      heading: "Tu contenido",
      blocks: [
        "Los archivos que subes y los videos que haces son tuyos. Nos autorizas a guardarlos, procesarlos y enviarlos a los proveedores solo en la medida necesaria para prestar el servicio, como describe la Política de privacidad.",
        "Declaras tener los derechos necesarios sobre lo que subes, incluidas la imagen y la voz de las personas que aparecen en tus archivos. No está permitido usar scorsese para:",
        [
          "contenido ilegal, o que sexualice a menores de cualquier forma;",
          "imitar a una persona real sin su permiso, incluida su voz, o engañar a alguien sobre quién habla o aparece en un video;",
          "violar derechos de autor, de marca o de imagen de terceros;",
          "acoso, discurso de odio o incitación a la violencia;",
          "intentar eludir los límites, los cobros o la seguridad del servicio.",
        ],
        "Los proveedores de inteligencia artificial tienen sus propias reglas y pueden rechazar un pedido. Podemos suspender o cerrar una cuenta que incumpla estos términos.",
      ],
    },
    {
      heading: "Resultados de inteligencia artificial",
      blocks: [
        "Lo que genera la inteligencia artificial puede salir distinto de lo que pediste, o contener errores. Revisa el video antes de publicarlo o usarlo comercialmente. Eres responsable del uso que hagas de él.",
      ],
    },
    {
      heading: "Disponibilidad",
      blocks: [
        "scorsese corre en un servidor propio y puede quedar fuera de línea por mantenimiento o por una falla. Hacemos copias de seguridad, pero te recomendamos guardar también una copia de los archivos y videos que sean importantes.",
        "En la medida en que la ley lo permita, nuestra responsabilidad por daños relacionados con el servicio se limita al valor que pagaste en los 12 meses anteriores.",
      ],
    },
    {
      heading: "Cierre de la cuenta",
      blocks: [
        `Puedes pedir el cierre de tu cuenta en cualquier momento, en ${SELLER.email}. La cuenta, los proyectos y los archivos se borran en un máximo de 15 días. El saldo restante se pierde, salvo lo que esté dentro del plazo de arrepentimiento descrito arriba, que se devuelve.`,
      ],
    },
    {
      heading: "Cambios en estos términos",
      blocks: [
        "Si cambiamos estos términos de forma relevante, avisaremos por correo o en el sitio con al menos 15 días de anticipación. Seguir usando el servicio después significa aceptar la nueva versión.",
      ],
    },
    {
      heading: "Si no vives en Brasil",
      blocks: [
        "Puedes usar scorsese desde cualquier país. Los créditos se cobran en reales y tu banco puede convertir el monto y cobrar comisiones propias. Los derechos que la ley de consumo de tu país te garantiza y que un contrato no puede excluir siguen vigentes.",
      ],
    },
    {
      heading: "Ley aplicable y jurisdicción",
      blocks: [
        "Estos términos se rigen por la ley brasileña. Se elige el foro de Brasilia, Distrito Federal, sin perjuicio de tu derecho, como consumidor, de demandar ante los tribunales de tu domicilio.",
      ],
    },
  ],
};
