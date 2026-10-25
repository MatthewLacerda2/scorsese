// The landing page at `/` (#904), for a visitor without a session. Written to
// a business that needs a video and has never heard of an editor: what it
// makes and how. Never JSON, MCP, API
// keys or a provider's name (CLAUDE.md, *The user never deals with
// implementation*).

export const landing = {
  meta: {
    title: "scorsese · The video editor you talk to",
    description:
      "Promos, ads and event videos for your business, made from your files and a description. No editor to hire, no software to learn.",
  },
  openApp: "Open scorsese",
  signIn: "Sign in",
  hero: {
    title: "The video is one conversation away",
    titleB: "Describe the video. We'll make it.",
    free: (dollars?: string) =>
      dollars ? `Start free: US$${dollars} a day, no card.` : "Start free, no card.",
  },
  how: {
    eyebrow: "How it works",
    title: "Three steps. You just say what you want.",
    files: {
      title: "Send your files",
      body: "Photos, clips, your logo, the song you like. Whatever you have, as it is.",
    },
    describe: {
      title: "Describe the video",
      body: "Say what it is for and how it should feel, in your own words, the way you'd brief a person.",
    },
    adjust: {
      title: "Watch it, then adjust",
      body: "See the cut, ask for changes in a sentence, and download it when it's right.",
    },
  },
  showcase: {
    eyebrow: "Made with scorsese",
    label: "Videos made with scorsese",
    soundOn: "Turn the sound on",
    soundOff: "Turn the sound off",
    volume: "Volume",
    play: "Play",
    pause: "Pause",
  },
  handles: {
    eyebrow: "What it handles for you",
    title: "Everything an editor would do.",
    music: {
      title: "Music",
      body: "A score written to the cut, or your song, sitting under everything else.",
    },
    titles: { title: "Titles", body: "Type that moves, in the fonts and colours of your brand." },
    cuts: {
      title: "Cuts",
      body: "The best moments of your footage, in an order that tells the story.",
    },
    pacing: { title: "Pacing", body: "Faster or slower, shorter or longer, in one sentence." },
    narration: {
      title: "Narration",
      body: "A voice that reads your script, timed to the picture.",
    },
  },
  ask: { title: "Your turn. Describe the video you need." },
  closing: {
    title: "Ready when you are.",
    body: "The video is one message away",
  },
  footer: "Every film on this page was made with scorsese.",
  dialog: {
    title: "Welcome to scorsese",
    description: "Sign in to your projects and files, or make your account.",
  },
  legal: {
    terms: "Terms of use",
    privacy: "Privacy policy",
    pricingShort: "Pricing",
    translation:
      "This is a translation. The Portuguese text is the binding one: if the versions disagree, the Portuguese text prevails.",
    readPortuguese: "Read the Portuguese text",
    readTranslation: "Back to the translation",
  },
};
