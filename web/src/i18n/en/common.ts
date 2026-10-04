// Strings the whole app shares — the header, Settings, the login page, the
// session guard — in English, the source the other catalogues are typed against.

export const common = {
  close: "Close",
  cancel: "Cancel",
  save: "Save",
  loading: "Loading…",
  tryAgain: "Try again",
  nav: { projects: "Projects", library: "Library" },
  balanceTitle: "Your credits — see what you spent",
  account: "Your account",
  spendingHistory: "Spending history",
  logOut: "Log out",
  settings: {
    open: "Settings",
    title: "Settings",
    description: "These are kept in this browser.",
    language: "Language",
    theme: "Theme",
  },
  theme: { light: "Light", dark: "Dark", system: "System" },
  login: {
    tagline: "Log in to your projects and files.",
    email: "Email",
    password: "Password",
    submit: "Log in",
    submitting: "Logging in…",
    invitation: "Accounts are made by invitation. Ask whoever gave you access for a password.",
  },
  unreachable: (error: string) => `Could not reach the server: ${error}`,
  notFound: { text: "There is nothing here.", link: "Go to your projects" },
};
