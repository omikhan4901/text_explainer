/** A keyboard shortcut as key caps: "Ctrl+Shift+Space" → Ctrl Shift Space. */
export function Kbd({ keys }: { keys: string }) {
  return (
    <span className="inline-flex items-center gap-1 align-middle">
      {keys.split("+").map((k) => (
        <kbd key={k} className="rounded-md border border-border-strong bg-surface-2 px-1.5 py-0.5 font-sans text-xs font-medium text-text shadow-[0_1px_0_var(--border-strong)]">
          {k}
        </kbd>
      ))}
    </span>
  );
}
