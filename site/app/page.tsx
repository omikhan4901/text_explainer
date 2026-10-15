import Image from "next/image";
import { Icon, type IconName } from "./icons";
import { Reveal } from "./reveal";

const DOWNLOAD = "https://github.com/omikhan4901/text_explainer/releases/download/latest-build/Plainly-x64-setup.exe";
const REPO = "https://github.com/omikhan4901/text_explainer";

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

function Keys({ keys, big = false }: { keys: string[]; big?: boolean }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      {keys.map((k, i) => (
        <kbd
          key={i}
          className={`inline-grid place-items-center rounded-md border border-line bg-surface font-medium text-text shadow-[0_1px_0_var(--line)] ${
            big ? "h-12 min-w-12 px-3.5 text-[17px]" : "h-8 min-w-8 px-2 text-[13px]"
          }`}
        >
          {k}
        </kbd>
      ))}
    </span>
  );
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
            <div className="mx-auto max-w-[480px] overflow-hidden rounded-2xl border border-line bg-[#e9e9ec] shadow-[var(--shadow)]">
              <Image
                src="/assets/demo.gif"
                alt="The Plainly card: it opens on a web search, then Explain with AI rewrites a lease clause in plain English, then translates it into Bangla"
                width={600}
                height={651}
                priority
                unoptimized
                className="h-auto w-full"
              />
            </div>
          </div>
        </section>

        {/* The moment: a real clause, its jargon marked, then the plain version. */}
        <section id="how" className="scroll-mt-16 border-t border-line bg-surface">
          <Reveal className="mx-auto max-w-4xl px-5 py-24 md:px-8 md:py-32" threshold={0.35}>
            <h2 className="max-w-[18ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">Select. Press one key. Understand.</h2>
            <div className="mt-14 space-y-8">
              <div>
                <p className="mb-3 text-[13px] font-medium text-muted">A clause from a lease</p>
                <p className="text-[22px] leading-[1.55] text-muted md:text-[26px]">
                  <span className="mark mark-full sweep">Notwithstanding any provision of this Agreement to the contrary</span>, the Lessee shall{" "}
                  <span className="mark mark-full sweep" style={{ transitionDelay: "0.9s" }}>
                    remit payment
                  </span>{" "}
                  of the monthly rent of $1,200 no later than the 5th day of each calendar month,{" "}
                  <span className="mark mark-full sweep" style={{ transitionDelay: "1.2s" }}>
                    failing which
                  </span>{" "}
                  a late fee equivalent to 5% of the outstanding amount{" "}
                  <span className="mark mark-full sweep" style={{ transitionDelay: "1.5s" }}>
                    shall accrue
                  </span>
                  .
                </p>
              </div>
              <div className="flex items-center gap-4 text-muted">
                <Keys keys={["Ctrl", "Shift", "Space"]} />
                <span className="h-px flex-1 bg-line" />
              </div>
              <div className="plain-swap">
                <p className="mb-3 text-[13px] font-medium text-muted">On the card, at the Plain level</p>
                <p className="text-[26px] leading-[1.45] font-medium tracking-[-0.015em] md:text-[32px]">
                  Whatever else this agreement says, the tenant must pay the monthly rent of $1,200 by the 5th day of each month. If they don&apos;t, a late fee of 5% of the
                  unpaid amount is added.
                </p>
                <p className="mt-5 text-[14px] text-muted">If an answer drops or invents a number, date or name, the card tells you.</p>
              </div>
            </div>
          </Reveal>
        </section>

        {/* What's in the card: four real screens, in an uneven grid. */}
        <section id="features" className="mx-auto max-w-6xl px-5 py-24 md:px-8 md:py-32">
          <Reveal>
            <h2 className="max-w-[20ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">Everything in one small card.</h2>
          </Reveal>
          <div className="mt-14 grid gap-4 md:grid-cols-12">
            <Feature
              i={0}
              span="md:col-span-7"
              image="feature-explain.png"
              title="Explain in plain words"
              body="Simpler, Plain or Clearer. What you selected stays on top, the plain version underneath."
              alt="The card explaining a lease clause: the original above, the plain English version below, with the reading grade going from 17 to 9"
            />
            <Feature
              i={1}
              span="md:col-span-5"
              image="feature-translate.png"
              title="Translate privately"
              body="Into Bangla, Hindi, Spanish, Arabic, Chinese and more, by the AI on your computer."
              alt="The card translating the lease clause into Bangla, with the language as a dropdown"
            />
            <Feature
              i={2}
              span="md:col-span-5"
              image="feature-lookup.png"
              title="Look up anything"
              body="Heard a word you don't know? Press the shortcut with nothing selected and type it."
              alt="The Look up box with the word serendipity typed in"
            />
            <Feature
              i={3}
              span="md:col-span-7"
              image="feature-words.png"
              title="Keep your words"
              body="Save words from the card, hear them read aloud, export them to Excel or Anki."
              alt="The Words page: saved words with their meanings, Listen, Remove, and export buttons"
            />
          </div>
        </section>

        {/* Two ways in. */}
        <section className="border-y border-line bg-surface">
          <div className="mx-auto grid max-w-6xl items-center gap-12 px-5 py-24 md:px-8 md:py-28 lg:grid-cols-2">
            <Reveal>
              <h2 className="max-w-[16ch] text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">Works wherever you can select text.</h2>
              <p className="mt-6 max-w-[44ch] text-[18px] leading-relaxed text-muted">
                PDFs, Word, email, chat, websites. Web searches and a dictionary come first; the AI only runs when you ask it to.
              </p>
            </Reveal>
            <Reveal className="space-y-8 lg:justify-self-end" style={{ ["--i" as string]: 1 }}>
              <div>
                <p className="mb-3 text-[14px] text-muted">Select, then press</p>
                <Keys keys={["Ctrl", "Shift", "Space"]} big />
              </div>
              <div>
                <p className="mb-3 text-[14px] text-muted">Or copy it twice</p>
                <Keys keys={["Ctrl", "C", "C"]} big />
              </div>
            </Reveal>
          </div>
        </section>

        {/* Privacy. */}
        <section id="private" className="mx-auto grid max-w-6xl gap-14 px-5 py-24 md:px-8 md:py-32 lg:grid-cols-[0.9fr_1.1fr]">
          <Reveal>
            <h2 className="text-4xl leading-[1.05] font-semibold tracking-[-0.03em] md:text-5xl">
              Private <span className="mark sweep">by design.</span>
            </h2>
            <p className="mt-6 max-w-[38ch] text-[18px] leading-relaxed text-muted">The AI and the dictionary run on your computer. What you read with them never leaves it.</p>
          </Reveal>
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

        {/* Download. */}
        <section className="mx-auto max-w-3xl px-5 py-28 text-center md:px-8 md:py-36">
          <Reveal>
            <Image src="/assets/logo.png" alt="" width={72} height={72} className="mx-auto" />
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

function Feature({ i, span, image, title, body, alt }: { i: number; span: string; image: string; title: string; body: string; alt: string }) {
  return (
    <Reveal className={`reveal group flex flex-col overflow-hidden rounded-2xl border border-line bg-surface ${span}`} style={{ ["--i" as string]: i }}>
      <div className="px-7 pt-7 pb-5">
        <h3 className="text-[19px] font-semibold tracking-[-0.01em]">{title}</h3>
        <p className="mt-1.5 max-w-[46ch] text-[15px] leading-relaxed text-muted">{body}</p>
      </div>
      {/* The screenshots sit on this same grey, so it fills the cell edge to edge. */}
      <div className="grid flex-1 place-items-center overflow-hidden bg-[#ececee]">
        <Image
          src={`/assets/${image}`}
          alt={alt}
          width={1200}
          height={840}
          className="h-auto w-full transition-transform duration-700 ease-[var(--ease-out)] group-hover:scale-[1.015]"
        />
      </div>
    </Reveal>
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
        <span className="block text-[44px] leading-none font-semibold tracking-[-0.04em] tabular-nums">{value}</span>
        <span className="mt-3 block max-w-[26ch] text-[15px] leading-relaxed text-muted">{label}</span>
      </dd>
    </Reveal>
  );
}
