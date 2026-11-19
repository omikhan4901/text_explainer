import type { MetadataRoute } from "next";

export const dynamic = "force-static";

export default function sitemap(): MetadataRoute.Sitemap {
  // Built statically, so this is the date of the last deploy.
  return [{ url: "https://plainly.omikhan.com/", lastModified: new Date(), changeFrequency: "weekly", priority: 1 }];
}
