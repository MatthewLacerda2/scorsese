import type { Messages } from "@/i18n/catalogue";

// The shared prompt box, on the landing page and the signed-in home.

export const prompt: Messages["prompt"] = {
  // Typed into the empty box one after another (`useTypewriter`), at most 50 characters each, the kinds of
  // video a person with an idea and some footage would ask for.
  examples: [
    "Video de lanzamiento de mis zapatillas",
    "Montaje del cumpleaños 60 de mi mamá",
    "Video explicativo narrado sobre nuestra app",
    "Recorrido por un departamento de dos habitaciones",
    "Lo mejor de la boda, con los votos de fondo",
    "Promo de 15 s de la oferta de mi panadería",
    "Resumen del viaje con subtítulos por día",
    "Intro para mi canal de cocina",
  ],
  label: "Describe tu video",
  send: "Enviar",
};
