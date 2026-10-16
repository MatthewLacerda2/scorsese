import type { Messages } from "@/i18n/catalogue";

export const common: Messages["common"] = {
  close: "Cerrar",
  cancel: "Cancelar",
  save: "Guardar",
  loading: "Cargando…",
  tryAgain: "Reintentar",
  nav: { projects: "Proyectos", library: "Biblioteca" },
  balanceTitle: "Tus créditos — mira en qué los gastaste",
  yourCredits: "Tus créditos",
  account: "Tu cuenta",
  spendingHistory: "Historial de gastos",
  logOut: "Cerrar sesión",
  settings: {
    open: "Ajustes",
    title: "Ajustes",
    description: "Se guardan en este navegador.",
    language: "Idioma",
    theme: "Tema",
  },
  theme: { light: "Claro", dark: "Oscuro", system: "Sistema" },
  login: {
    google: "Continuar con Google",
    newHere: "¿Primera vez? Tu primer inicio de sesión crea tu cuenta.",
    accept: {
      before: "Al continuar, aceptas los ",
      terms: "términos de uso",
      between: " y la ",
      privacy: "política de privacidad",
      after: ". Debes tener 18 años o más.",
    },
    errors: {
      cancelled: "Cancelaste el inicio de sesión en Google. Inténtalo de nuevo cuando quieras.",
      expired: "Ese inicio de sesión tardó demasiado o empezó en otra pestaña. Inténtalo de nuevo.",
      unverified:
        "Google todavía no verificó el correo de esta cuenta, así que no puede iniciar sesión.",
      linked:
        "La cuenta de este correo está vinculada a otra cuenta de Google. Entra con esa o pídenos que vinculemos esta.",
      unconfigured: "El inicio de sesión todavía no está configurado en este servidor.",
      failed: "Algo salió mal al iniciar sesión. Inténtalo de nuevo.",
    },
  },
  crashed: { text: "Algo salió mal en esta página.", reload: "Recargar" },
  unreachable: (error: string) => `No se pudo conectar con el servidor: ${error}`,
  notFound: { text: "Aquí no hay nada.", link: "Ir a tus proyectos" },
};
