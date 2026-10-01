// Every UI string. Keys are grouped by screen.
export const en = {
  app: {
    name: "Text Explainer",
    tagline: "Select text anywhere, get a readable version. Fully offline.",
  },
  level: {
    label: "Reading level",
    simpler: "Simpler",
    plain: "Plain",
    clearer: "Clearer",
    simplerHint: "Very short sentences and everyday words (about grade 6).",
    plainHint: "Plain language anyone can follow (about grade 8–9).",
    clearerHint: "Same level and terms, untangled.",
  },
  card: {
    close: "Close (Esc)",
    copy: "Copy",
    copied: "Copied",
    pin: "Keep open",
    unpin: "Close when I click elsewhere",
    grade: (before: number, after: number) => `Grade ${Math.round(before)} → ${Math.round(after)}`,
    gradeTitle: "Reading grade before and after (Flesch–Kincaid).",
    loadingModel: "Loading the model. The first time after a while takes a few seconds.",
    thinking: "Reading…",
    truncated: "This was long, so only the first few pages were explained.",
    checkMissing: "Not found in the rewrite:",
    checkAdded: "Not in the original:",
    checkHint: "Check these against the original text.",
    meaningOf: "Meaning",
    noSelectionTitle: "Nothing selected",
    noSelection: (hotkey: string) => `Select some text, then press ${hotkey}. Or copy it twice with Ctrl+C, C.`,
    noModelTitle: "Choose a model first",
    noModel: "Text Explainer needs a model to read with. It takes a minute to set up.",
    setUp: "Set up",
    errorTitle: "Something went wrong",
    details: "Details",
    settings: "Settings",
    captureFailed: "Couldn't read the selection.",
    retry: "Try again",
  },
} as const;

export type Strings = typeof en;
