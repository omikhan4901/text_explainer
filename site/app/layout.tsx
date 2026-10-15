import type { Metadata, Viewport } from "next";
import localFont from "next/font/local";
import "./globals.css";

const geist = localFont({
  src: [
    { path: "./fonts/geist.woff2", style: "normal", weight: "100 900" },
    { path: "./fonts/geist-italic.woff2", style: "italic", weight: "100 900" },
  ],
  variable: "--font-geist",
  display: "swap",
});
const geistMono = localFont({ src: "./fonts/geist-mono.woff2", variable: "--font-geist-mono", display: "swap", weight: "100 900" });

const SITE = "https://plainly.omikhan.com";
const TITLE = "Plainly: hard text in plain words, on Windows";
const DESCRIPTION =
  "Select hard text in any Windows app and read it in plain words, right next to it. A local AI explains and translates it on your own computer. Free preview.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE),
  title: TITLE,
  description: DESCRIPTION,
  applicationName: "Plainly",
  authors: [{ name: "Omi Khan", url: "https://github.com/omikhan4901" }],
  keywords: ["plain language", "explain text", "translate privately", "offline AI", "Windows app", "dictionary", "reading help", "local LLM"],
  alternates: { canonical: "/" },
  icons: { icon: "/assets/favicon.png", apple: "/assets/apple-touch-icon.png" },
  openGraph: {
    type: "website",
    url: SITE,
    siteName: "Plainly",
    title: TITLE,
    description: DESCRIPTION,
  },
  twitter: { card: "summary_large_image", title: TITLE, description: DESCRIPTION },
  robots: { index: true, follow: true },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f5f5f4" },
    { media: "(prefers-color-scheme: dark)", color: "#131312" },
  ],
};

/** Structured data: tells search engines this is a free Windows application. */
const softwareJsonLd = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "Plainly",
  operatingSystem: "Windows 10, Windows 11",
  applicationCategory: "EducationalApplication",
  description: DESCRIPTION,
  url: SITE,
  downloadUrl: "https://github.com/omikhan4901/text_explainer/releases/download/latest-build/Plainly-x64-setup.exe",
  offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
  author: { "@type": "Person", name: "Omi Khan" },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${geist.variable} ${geistMono.variable}`}>
      <body className="font-sans antialiased">
        <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(softwareJsonLd) }} />
        {children}
      </body>
    </html>
  );
}
