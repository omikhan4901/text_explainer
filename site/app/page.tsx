import Image from "next/image";
import { AppsSection } from "./demo/apps";
import { CheckSection } from "./demo/check";
import { CountValue } from "./demo/count-up";
import { HeroDemo } from "./demo/hero-demo";
import { LevelsSection } from "./demo/levels";
import { LogoDraw } from "./demo/logo-draw";
import { MorphSection } from "./demo/morph";
import { PrivateMark } from "./demo/private-mark";
import { TranslateSection } from "./demo/translate";
import { WordsSection } from "./demo/words";
import { Icon, type IconName } from "./icons";
import { Reveal } from "./reveal";

const DOWNLOAD = "https://github.com/omikhan4901/text_explainer/releases/download/latest-build/Plainly-x64-setup.exe";
const REPO = "https://github.com/omikhan4901/text_explainer";

// Every answer is something the README says is built today.
const FAQ: [string, string][] = [
  ["Is Plainly free?", "Yes. It's a free preview for Windows 10 and 11, and its source code is open under the MIT licence."],
  [
    "Does my text leave my computer?",
    "Not for the AI or the dictionary: both run on your own computer. The web tabs (Google, Bing, Wikipedia and others) are like visiting those sites and load only when you open them. Offline only turns them all off.",
  ],
  ["What do I need to run it?", "A 64-bit Windows 10 or 11 PC. The default model is comfortable on 8 GB of memory and needs no graphics card. There's no account to create."],
  [
    "Which apps does it work in?",
    "Anywhere you can select text: PDF readers, Word, email, chat apps and browsers. Select, then press Ctrl+Shift+Space, or press Ctrl+C twice. It never reads text in password managers or in apps you add to its list.",
  ],
  [
    "Which languages can it translate?",
    "The AI on your computer translates into languages such as Bangla, Hindi, Urdu, Arabic, Spanish, French, German, Chinese and Japanese. How well depends on the model and the language.",
  ],
  ["Is it on Mac or Linux?", "Not yet. Plainly is Windows only for now."],
];

function DownloadButton({ size = "md" }: { size?: "sm" | "md" }) {
  const box = size === "sm" ? "h-9 px-3.5 text-[13.5px]" : "h-12 px-5 text-[15px]";
  return (
    <a
      href={DOWNLOAD}
      className={`press inline-flex shrink-0 items-center gap-2 rounded-lg bg-ink font-medium whitespace-nowrap text-on-ink hover:bg-ink-hover ${box}`}
    >
      <Icon name="download" size={size === "sm" ? 15 : 18} />
      Download for Windows
    </a>
  );
}

function Mark({ size = 26 }: { size?: number }) {
  return <Image src="/assets/logo.png" alt="" width={size} height={size} className="shrink-0" priority />;
}

export default function Home() {
  return (
    <>
      <a href="#main" className="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-50 focus:rounded-md focus:bg-ink focus:px-3 focus:py-2 focus:text-on-ink">
        Skip to content
      </a>

      <header className="sticky top-0 z-40 border-b border-line bg-bg/80 backdrop-blur-md">
        <nav className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-6 px-5 md:px-8" aria-label="Main">
          <a href="#top" className="flex items-center gap-2.5 text-[17px] font-semibold tracking-tight">
            <Mark />
            Plainly
          </a>
          <div className="flex items-center gap-7">
            <a href="#how" className="hidden text-[14px] text-muted transition-colors hover:text-text md:block">
              How it works
            </a>
            <a href="#private" className="hidden text-[14px] text-muted transition-colors hover:text-text md:block">
              Privacy
            </a>
            <a href={REPO} className="hidden text-[14px] text-muted transition-colors hover:text-text md:block">
              GitHub
            </a>
            <DownloadButton size="sm" />
          </div>
        </nav>
      </header>

      <main id="main">
        {/* Hero: the promise on the left, the real card on the right. */}
        <section id="top" className="mx-auto grid max-w-6xl items-center gap-14 px-5 pt-14 pb-20 md:px-8 md:pt-20 lg:grid-cols-[1.05fr_0.95fr] lg:gap-10 lg:pb-28">
          <div>
            <h1 className="rise text-[44px] leading-[1.02] font-semibold tracking-[-0.035em] sm:text-6xl lg:text-[68px]" style={{ ["--i" as string]: 0 }}>
              Hard text,
              <br />
              <span className="mark draw">in plain words.</span>
            </h1>
            <p className="rise mt-6 max-w-[34ch] text-[19px] leading-relaxed text-muted" style={{ ["--i" as string]: 1 }}>
              Select hard text in any Windows app. Plainly explains or translates it beside your cursor, on your own computer.
            </p>
            <div className="rise mt-9 flex flex-wrap items-center gap-3" style={{ ["--i" as string]: 2 }}>
              <DownloadButton />
              <a href="#how" className="press inline-flex h-12 items-center gap-2 rounded-lg px-4 text-[15px] font-medium text-text hover:bg-sunken">
                See how it works
                <Icon name="arrow" size={16} />
              </a>
            </div>
          </div>
          <div className="rise relative" style={{ ["--i" as string]: 2 }}>
            <div className="mx-auto w-full max-w-[600px]">
              <HeroDemo />
            </div>
          </div>
        </section>

        <MorphSection />
        <LevelsSection />
        <AppsSection />
        <TranslateSection />
        <WordsSection />
        <CheckSection />

        {/* Privacy. */}
        <section id="private" className="mx-auto grid max-w-6xl gap-14 px-5 py-24 md:px-8 md:py-32 lg:grid-cols-[0.9fr_1.1fr]">
          <div>
            <PrivateMark />
            <Reveal>
              <h2 className="text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
                Private <span className="mark sweep">by design.</span>
              </h2>
              <p className="mt-6 max-w-[38ch] text-[18px] leading-relaxed text-muted">The AI and the dictionary run on your computer. What you read with them never leaves it.</p>
            </Reveal>
          </div>
          <div className="grid gap-x-10 gap-y-10 sm:grid-cols-2">
            <Point i={0} icon="cpu" title="Runs on your computer" body="A small AI model on an ordinary laptop. No account, no cloud, no subscription to read." />
            <Point i={1} icon="globe" title="Online only when you choose" body="A web tab loads only when you open it. One switch turns them all off." />
            <Point i={2} icon="shield" title="Stays out of your passwords" body="It never reads text in password managers, or in any app you add to the list." />
            <Point i={3} icon="search" title="Any model you like" body="It recommends one that fits your PC, or uses one you already have from LM Studio or Ollama." />
          </div>
        </section>

        {/* Measured, not guessed. */}
        <section className="border-t border-line bg-surface">
          <div className="mx-auto max-w-6xl px-5 py-24 md:px-8 md:py-28">
            <Reveal>
              <h2 className="max-w-[22ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">Fast enough on an ordinary laptop.</h2>
            </Reveal>
            <dl className="mt-14 grid gap-x-8 gap-y-12 sm:grid-cols-2 lg:grid-cols-4">
              <Stat i={0} value="9.2" label="reading grade levels easier, on average, at the Plain level" />
              <Stat i={1} value="2.2 s" label="to the first words, on a 4-core CPU with no graphics card" />
              <Stat i={2} value="3.9 GB" label="peak memory for the default model, comfortable on 8 GB" />
              <Stat i={3} value="0 of 56" label="answers in the wrong language, kept in line token by token" />
            </dl>
            <p className="mt-12 text-[14px] text-muted">
              Measured with the default model, Gemma 4 E2B, on a 4-core machine with no GPU.{" "}
              <a href={`${REPO}/blob/main/docs/models.md`} className="underline underline-offset-4 hover:text-text">
                See the full results
              </a>
              .
            </p>
          </div>
        </section>

        {/* Questions people search for, answered from the README. Also as FAQPage data. */}
        <section id="faq" className="scroll-mt-16 border-t border-line">
          <div className="mx-auto max-w-3xl px-5 py-24 md:px-8 md:py-28">
            <Reveal>
              <h2 className="text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">Questions</h2>
            </Reveal>
            <div className="mt-10 divide-y divide-line border-y border-line">
              {FAQ.map(([q, a]) => (
                <details key={q} className="group py-5">
                  <summary className="flex cursor-pointer list-none items-center justify-between gap-6 text-[18px] font-medium">
                    {q}
                    <span aria-hidden="true" className="text-muted transition-transform duration-300 group-open:rotate-45">
                      +
                    </span>
                  </summary>
                  <p className="mt-3 max-w-[60ch] text-[16px] leading-relaxed text-muted">{a}</p>
                </details>
              ))}
            </div>
          </div>
          <script
            type="application/ld+json"
            dangerouslySetInnerHTML={{
              __html: JSON.stringify({
                "@context": "https://schema.org",
                "@type": "FAQPage",
                mainEntity: FAQ.map(([q, a]) => ({ "@type": "Question", name: q, acceptedAnswer: { "@type": "Answer", text: a } })),
              }),
            }}
          />
        </section>

        {/* Download. */}
        <section className="mx-auto max-w-3xl px-5 py-28 text-center md:px-8 md:py-36">
          <LogoDraw />
          <Reveal>
            <h2 className="mt-8 text-5xl leading-[1.02] font-semibold tracking-[-0.035em] md:text-6xl">
              Read anything, <span className="mark sweep">plainly.</span>
            </h2>
            <p className="mt-6 text-[18px] text-muted">Free preview for Windows 10 and 11.</p>
            <div className="mt-9 flex justify-center">
              <DownloadButton />
            </div>
            <p className="mx-auto mt-6 max-w-[52ch] text-[13.5px] leading-relaxed text-muted">
              The installer isn&apos;t code-signed yet, so Windows may warn you. Choose More info, then Run anyway.
            </p>
          </Reveal>
        </section>
      </main>

      <footer className="border-t border-line">
        <div className="mx-auto flex max-w-6xl flex-col items-start justify-between gap-4 px-5 py-8 text-[14px] text-muted sm:flex-row sm:items-center md:px-8">
          <span className="flex items-center gap-2.5">
            <Mark size={20} />
            <span>Plainly, by Omi Khan, 2026</span>
          </span>
          <span className="flex items-center gap-6">
            <a href={REPO} className="inline-flex items-center gap-1.5 hover:text-text">
              <Icon name="github" size={16} />
              Source on GitHub
            </a>
            <a href={`${REPO}/blob/main/LICENSE`} className="hover:text-text">
              MIT licence
            </a>
          </span>
        </div>
      </footer>
    </>
  );
}

function Point({ i, icon, title, body }: { i: number; icon: IconName; title: string; body: string }) {
  return (
    <Reveal className="reveal" style={{ ["--i" as string]: i }}>
      <span className="grid size-10 place-items-center rounded-lg bg-sunken text-text">
        <Icon name={icon} size={20} />
      </span>
      <h3 className="mt-4 text-[17px] font-semibold tracking-[-0.01em]">{title}</h3>
      <p className="mt-1.5 text-[15px] leading-relaxed text-muted">{body}</p>
    </Reveal>
  );
}

function Stat({ i, value, label }: { i: number; value: string; label: string }) {
  return (
    <Reveal className="reveal border-t border-line pt-5" style={{ ["--i" as string]: i }}>
      <dt className="sr-only">{label}</dt>
      <dd>
        <CountValue value={value} className="block text-[44px] leading-none font-semibold tracking-[-0.04em] tabular-nums" />
        <span className="mt-3 block max-w-[26ch] text-[15px] leading-relaxed text-muted">{label}</span>
      </dd>
    </Reveal>
  );
}
