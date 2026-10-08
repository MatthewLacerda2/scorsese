// The assistant's panel in the editor, in English — the source the other
// catalogues are typed against. What the assistant writes and what the server
// says stay in their own words; this is only the panel around them.

export const chat = {
  loading: "Loading…",
  jobs: {
    veo_shot: "Video shot",
    still_image: "Still",
    spoken_line: "Spoken line",
    voice_design: "Voice design",
  },
  jobState: {
    waiting: "queued",
    running: "generating",
    done: "ready",
    failed: (error: string | null) => (error ? `failed: ${error}` : "failed"),
    stuck: (error: string | null) =>
      error ? `stuck waiting on the provider: ${error}` : "stuck waiting on the provider",
    cancelled: "cancelled",
  },
  ending: {
    refused: "The assistant declined this one.",
    capped: "Stopped: this turn reached what one turn may spend, or your credit ran out.",
    stopped: "Stopped, as you asked.",
    failed: "This turn failed.",
    interrupted: "The server restarted while this turn ran.",
  },
  problem: {
    unconfigured: "The assistant is not set up on this server yet.",
    noCredit: "You have no credit left for the assistant.",
    busy: "The assistant is still working on the last message.",
  },
  composer: {
    placeholder: "Ask the assistant… (Enter sends, Shift+Enter for a new line)",
    answerPlaceholder:
      "Answer the assistant's question — pick an option above, or write your own answer here",
    fresh: "New conversation",
    stop: "Stop",
    send: "Send",
  },
  turn: {
    busy: [
      "Splicing…",
      "Colour-grading…",
      "Rummaging through the footage…",
      "Lining up the cuts…",
      "Rewinding the tape…",
      "Syncing the sound…",
      "Trimming a few frames…",
      "Checking the take…",
      "Pulling focus…",
      "Setting up the shot…",
      "Cueing the music…",
      "Finding the right angle…",
    ],
    costSoFar: (cost: string) => `So far this turn has cost ${cost}; nothing more while it waits`,
    cost: (cost: string) => `This turn cost ${cost}`,
    left: (balance: string) => ` · ${balance} left`,
  },
  effort: {
    label: "How carefully the assistant works on this message",
    name: { low: "Quick", medium: "Balanced", high: "Thorough" },
    hint: {
      low: "For small tweaks, like making the title bigger or moving the music. Cheapest.",
      medium: "For ordinary requests.",
      high: "For building or reworking an edit. Takes longest and costs the most.",
    },
  },
  model: {
    label: "Model",
    unavailable: "unavailable",
    switchTitle: "Switch models?",
    switchWarning:
      "Switching models causes a cache miss on the next prompt. Caches store your conversation " +
      "to make it cheaper. Are you sure you want to switch?",
    switch: "Switch",
    cost: (words: string) => `Cost: ${words}`,
  },
  cost: {
    cheapest: "cheapest",
    inexpensive: "inexpensive",
    moderate: "moderately expensive",
    expensive: "expensive",
    dearest: "most expensive",
  },
  quote: {
    ask: (cost: string) => `This costs ${cost} from your credits. Go ahead?`,
    choose: (now: string, batch: string) =>
      `Now: ${now} · within 24 hours: ${batch}, half price for the stills. Which would you like?`,
    now: (cost: string) => `Now — ${cost}`,
    batch: (cost: string) => `Within 24 hours — ${cost}`,
    expired: "This quote has expired.",
    confirm: "Confirm",
    decline: "Decline",
    changePlaceholder: "Or say what to change — e.g. make the cape yellow instead of red",
    change: "Ask for a change",
    brief: { prompt: "Prompt", line: "Line", voice: "Voice" },
    less: "less",
    more: "more",
  },
  question: {
    notAnswered: "Not answered.",
    ownPlaceholder: "Or answer in your own words",
    answer: "Answer",
  },
  picker: {
    see: (count: number) => `See the ${count} options`,
    hint: "Click a picture to see it larger, tick the ones you like, and confirm.",
    ownPlaceholder: "Or say what you'd like instead — e.g. something darker",
    enlarge: "See it larger",
    select: "Select",
    selected: "Selected",
    back: "All options",
    by: (author: string) => `by ${author}`,
    from: (source: string) => `From ${source}`,
    none: "None of these",
    noneSay: "None — say this instead",
    use: (count: number) => (count > 1 ? `Use these ${count}` : "Use this one"),
    noneTaken: "None picked.",
  },
};
