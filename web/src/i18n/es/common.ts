import type { Messages } from "@/i18n/catalogue";

export const common: Messages["common"] = {
  close: "Cerrar",
  cancel: "Cancelar",
  save: "Guardar",
  loading: "Cargando…",
  tryAgain: "Reintentar",
  nav: { projects: "Proyectos", library: "Biblioteca" },
  balanceTitle: "Tus créditos — mira en qué los gastaste",
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
    email: "Correo electrónico",
    password: "Contraseña",
    submit: "Iniciar sesión",
    submitting: "Iniciando sesión…",
    invitation: "Las cuentas se crean por invitación. Pide una contraseña a quien te dio acceso.",
  },
  unreachable: (error: string) => `No se pudo conectar con el servidor: ${error}`,
  notFound: { text: "Aquí no hay nada.", link: "Ir a tus proyectos" },
};
