// The assistant's panel in the editor, in English — the source the other
// catalogues are typed against. What the assistant writes and what the server
// says stay in their own words; this is only the panel around them.

export const chat = {
  loading: "Loading…",
  /** The one line shown while any generation is running; the dots animate. */
  generating: "Generating",
  ending: {
    refused: "The assistant declined this one.",
    capped: "Stopped: this turn reached what one turn may spend, or your credit ran out.",
    stopped: "Stopped, as you asked.",
    failed:
      "This turn failed. If it keeps failing, start a new conversation: it carries on with the project as it is.",
    interrupted: "The server restarted while this turn ran.",
  },
  problem: {
    unconfigured: "The assistant is not set up on this server yet.",
    noCredit: "You have no credit left for the assistant.",
    busy: "The assistant is still working on the last message.",
    /** Files that did not come through the sign-in (#1535); their names follow. */
    missing: "These files did not come through the sign-in. Attach them again, then send:",
  },
  /** The chat taking the whole page, and back (#1027). */
  full: {
    enter: "Full",
    leave: "Back to the editor",
  },
  /** The Script button beside the pickers, and its modal (#1445). */
  script: {
    open: "Script",
    hint: "The script the assistant builds your video from",
    title: "Script",
    description:
      "What the assistant builds the video from. Change anything: it reads your version next time.",
    empty: "No script yet — the assistant writes one when you start a video.",
    edit: "Edit",
    write: "Write one",
    save: "Save",
    cancel: "Cancel",
    moved:
      "The assistant changed the script while you were editing, so yours was not saved. Here it is as it is now, with your version below to copy from.",
    yours: "Your version, not saved",
    changed:
      "The assistant has just changed the script. Saving won't write over it: your version is kept beside it to copy from.",
    conflict: "The project is changing right now; try saving again in a moment.",
    close: "Close",
    failed: "The script could not be saved.",
  },
  composer: {
    placeholder: "Ask the assistant… (Enter sends, Shift+Enter for a new line)",
    answerPlaceholder:
      "Answer the assistant's question — pick an option above, or write your own answer here",
    fresh: "New conversation",
    freshTitle: "Start a new conversation? This one stays saved.",
    freshStart: "Start",
    freshCancel: "Cancel",
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
    woken: "Carried on once your generations landed",
    left: (balance: string) => ` · ${balance} left`,
  },
  effort: {
    label: "How hard the assistant thinks",
    name: { low: "Low", medium: "Medium", high: "High", xhigh: "Extra" },
    hint: {
      low: "For small tweaks, like making the title bigger or moving the music. Cheapest.",
      medium: "For ordinary requests.",
      high: "For building or reworking an edit. Takes longer and costs more.",
      xhigh:
        "For the biggest jobs, like a whole video from one long brief. Takes longest and costs the most.",
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
    costLabel: "Cost",
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
    ask: (cost: string) => `This will cost ${cost} from your credits. Go ahead?`,
    choose: (now: string, batch: string) =>
      `Now: ${now} · within 24 hours: ${batch}, half price for the stills. Which would you like?`,
    now: (cost: string) => `Now — ${cost}`,
    batch: (cost: string) => `Within 24 hours — ${cost}`,
    expired: "This quote has expired.",
    confirm: "Confirm",
    decline: "Decline",
    changePlaceholder: "Change something here...",
    change: "Ask for a change",
    /** An answered quote, as the one line it collapses into. */
    answered: {
      confirmed: (cost: string) => `Approved — ${cost}`,
      declined: (cost: string) => `Declined — ${cost}`,
      changed: (cost: string) => `Change asked — ${cost}`,
      withdrawn: (cost: string) => `Not answered — ${cost}`,
    },
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
