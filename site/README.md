# Plainly website

The one-page site at **https://plainly.omikhan.com**, hosted on Vercel: Next.js (App
Router), pre-rendered to static HTML at build time, so every word is in the page for
search engines, with
metadata, structured data (a free Windows app), `sitemap.xml`, `robots.txt` and a social
preview image. Tailwind v4, Geist (self-hosted), Phosphor icons. Light and dark follow the
system; motion is off under "reduce motion". Checked with axe (WCAG 2.2 AA, both themes).

```bash
cd site
npm ci
npm run dev        # http://localhost:3000
npx next build     # the production build Vercel runs
```

`scripts/check.sh` type-checks and builds it when `site/node_modules` exists. Vercel
redeploys on every push that changes `site/` (`vercel.json` skips the rest); the Windows
installer build skips site-only changes.

## Hosting it on plainly.omikhan.com with Vercel (once)

1. **Import the repository.** vercel.com, **Add New... > Project**, pick
   `omikhan4901/text_explainer` (if it isn't listed, **Adjust GitHub App Permissions** and
   give Vercel access to it).
2. **Point it at the site.** On the import screen, **Root Directory > Edit > `site`**. The
   framework shows as Next.js; leave the build settings as they are. **Deploy**.
3. **Add the subdomain.** The new project, **Settings > Domains**, add
   `plainly.omikhan.com`.
   - If omikhan.com uses Vercel's nameservers, that's all: it works within a minute.
   - If its DNS is elsewhere, Vercel shows a record to add there: Type **CNAME**, Name
     **plainly**, Value **cname.vercel-dns.com** (use exactly what Vercel shows).
4. HTTPS is automatic once the domain checks out.

From then on, every push to `main` that changes `site/` redeploys it.
