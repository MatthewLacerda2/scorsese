import type { Messages } from "@/i18n/catalogue";

export const common: Messages["common"] = {
  close: "Fechar",
  cancel: "Cancelar",
  save: "Salvar",
  loading: "Carregando…",
  tryAgain: "Tentar de novo",
  nav: { projects: "Projetos", library: "Biblioteca" },
  balanceTitle: "Seus créditos — veja o que você gastou",
  account: "Sua conta",
  spendingHistory: "Histórico de gastos",
  logOut: "Sair",
  settings: {
    open: "Configurações",
    title: "Configurações",
    description: "Ficam salvas neste navegador.",
    language: "Idioma",
    theme: "Tema",
  },
  theme: { light: "Claro", dark: "Escuro", system: "Sistema" },
  login: {
    google: "Continuar com o Google",
    newHere: "Primeira vez? Seu primeiro acesso cria a sua conta.",
    errors: {
      cancelled: "O acesso foi cancelado no Google. Tente de novo quando quiser.",
      expired: "Esse acesso demorou demais ou começou em outra aba. Tente de novo.",
      unverified: "O Google ainda não verificou o e-mail desta conta, então ela não pode entrar.",
      linked:
        "A conta deste e-mail está ligada a outra conta do Google. Entre com aquela ou peça para ligarmos esta.",
      unconfigured: "O acesso ainda não está configurado neste servidor.",
      failed: "Algo deu errado ao entrar. Tente de novo.",
    },
  },
  unreachable: (error: string) => `Não foi possível falar com o servidor: ${error}`,
  notFound: { text: "Não tem nada aqui.", link: "Ir para seus projetos" },
};
