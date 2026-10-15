/** A keyboard shortcut as key caps: "Ctrl+Shift+Space" → Ctrl Shift Space. */
export function Kbd({ keys, size = "md" }: { keys: string; size?: "md" | "lg" }) {
  const cap = size === "lg" ? "min-w-8 px-2 py-1 text-[13px]" : "min-w-6 px-1.5 py-px text-[11px]";
  return (
    <span className="inline-flex items-center gap-1 align-middle">
      {keys.split("+").map((k) => (
        <kbd
          key={k}
          className={`inline-block rounded-[5px] border border-border-strong bg-surface text-center font-sans font-medium text-text shadow-[0_1px_0_var(--border-strong)] ${cap}`}
        >
          {k}
        </kbd>
      ))}
    </span>
  );
}
