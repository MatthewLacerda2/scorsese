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
    email: "E-mail",
    password: "Senha",
    submit: "Entrar",
    submitting: "Entrando…",
    invitation: "As contas são criadas por convite. Peça uma senha a quem te deu acesso.",
  },
  unreachable: (error: string) => `Não foi possível falar com o servidor: ${error}`,
  notFound: { text: "Não tem nada aqui.", link: "Ir para seus projetos" },
};
