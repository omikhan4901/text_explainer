// Every UI string. Keys are grouped by screen.
export const en = {
  app: {
    name: "Text Explainer",
    tagline: "Select text anywhere, get a readable version. Fully offline.",
  },
} as const;

export type Strings = typeof en;
