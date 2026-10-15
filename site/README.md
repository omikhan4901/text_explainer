# Plainly website

The one-page site at **https://plainly.omikhan.com**: Next.js (App Router) exported as
static HTML (`output: "export"`), so every word is in the page for search engines, with
metadata, structured data (a free Windows app), `sitemap.xml`, `robots.txt` and a social
preview image. Tailwind v4, Geist (self-hosted), Phosphor icons. Light and dark follow the
system; motion is off under "reduce motion". Checked with axe (WCAG 2.2 AA, both themes).

```bash
cd site
npm ci
npm run dev        # http://localhost:3000
npx next build     # static site in site/out
```

`scripts/check.sh` type-checks and builds it when `site/node_modules` exists. Pushing a
change under `site/` publishes it (`.github/workflows/site.yml`); the Windows installer
build skips site-only changes.

## Hosting it on plainly.omikhan.com (once)

1. **Turn on Pages.** GitHub, repository, **Settings > Pages**. Under "Build and
   deployment", set **Source** to **GitHub Actions**.
2. **Publish.** **Actions > Website > Run workflow** (or push any change under `site/`).
   Wait for the green tick.
3. **Point the subdomain at GitHub.** Where omikhan.com's DNS is managed (Cloudflare,
   Namecheap, Vercel, your registrar), add a record:
   - Type **CNAME**, Name/Host **plainly**, Value/Target **omikhan4901.github.io**, TTL Auto.
   - On Cloudflare, set the cloud to grey (**DNS only**), at least until step 5 is done.
4. **Tell GitHub the domain.** **Settings > Pages > Custom domain**: type
   `plainly.omikhan.com`, **Save**. Wait for "DNS check successful" (minutes, sometimes up
   to an hour).
5. **HTTPS.** When the certificate is ready, tick **Enforce HTTPS**.
6. **Optional, recommended:** verify the domain so nobody else can claim the subdomain:
   your GitHub profile **Settings > Pages > Add a domain**, then add the TXT record it shows.

Open the site at the custom domain; the `github.io` address isn't set up for it (assets
load from the root).
