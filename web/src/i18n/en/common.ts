// Strings the whole app shares — the header, Settings, the sign-in popup, the
// session guard — in English, the source the other catalogues are typed against.

export const common = {
  close: "Close",
  cancel: "Cancel",
  save: "Save",
  loading: "Loading…",
  tryAgain: "Try again",
  nav: { projects: "Projects", library: "Library" },
  balanceTitle: "Your credits — see what you spent",
  yourCredits: "Your credits",
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
  theme: { light: "Light", dark: "Dark" },
  login: {
    google: "Continue with Google",
    newHere: "New here? Your first sign-in makes your account.",
    accept: {
      before: "By continuing you accept the ",
      terms: "terms of use",
      between: " and the ",
      privacy: "privacy policy",
      after: ". You must be 18 or older.",
    },
    errors: {
      cancelled: "Sign-in was cancelled at Google. Try again whenever you like.",
      expired: "That sign-in took too long or was started elsewhere. Please try again.",
      unverified: "Google hasn't verified this account's email address yet, so it can't sign in.",
      linked:
        "This email's account is linked to a different Google account. Sign in with that one, or ask us to link this one.",
      unconfigured: "Sign-in isn't set up on this server yet.",
      failed: "Something went wrong signing you in. Please try again.",
    },
  },
  crashed: {
    text: "Something went wrong on this page.",
    part: "This part failed.",
    reload: "Reload",
  },
  unreachable: (error: string) => `Could not reach the server: ${error}`,
  notFound: { text: "There is nothing here.", link: "Go to your projects" },
};
