import type { NextConfig } from "next";

// Static export: every page is pre-rendered to HTML at build time (full content for search
// engines), then served as plain files by GitHub Pages.
const config: NextConfig = {
  output: "export",
  images: { unoptimized: true },
  trailingSlash: true,
  poweredByHeader: false,
};

export default config;
