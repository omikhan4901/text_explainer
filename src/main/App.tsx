import { strings } from "../i18n";

export function App() {
  return (
    <main className="mx-auto max-w-2xl p-10">
      <h1 className="text-3xl font-semibold">{strings.app.name}</h1>
      <p className="mt-2 text-muted">{strings.app.tagline}</p>
    </main>
  );
}
