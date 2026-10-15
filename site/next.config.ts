import type { NextConfig } from "next";

// Hosted on Vercel (site/README.md). The page is static: it is pre-rendered to HTML at
// build time, so search engines get every word without running any JavaScript.
const config: NextConfig = {
  poweredByHeader: false,
};

export default config;
