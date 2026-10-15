import { ToggleGroup } from "radix-ui";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  title?: string;
  disabled?: boolean;
}

/** A compact one-of-several control. */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
  size = "md",
}: {
  value: T;
  options: SegmentedOption<T>[];
  onChange: (value: T) => void;
  label: string;
  size?: "sm" | "md";
}) {
  const pad = size === "sm" ? "h-6 px-2 text-[12px]" : "h-7 px-3 text-[13px]";
  return (
    <ToggleGroup.Root
      type="single"
      value={value}
      aria-label={label}
      onValueChange={(v) => v && onChange(v as T)}
      className="inline-flex rounded-md bg-surface-2 p-0.5"
    >
      {options.map((o) => (
        <ToggleGroup.Item
          key={o.value}
          value={o.value}
          title={o.title}
          disabled={o.disabled}
          className={`${pad} rounded-[5px] font-medium text-muted transition-colors hover:text-text disabled:cursor-default disabled:opacity-50 disabled:hover:text-muted data-[state=on]:bg-surface data-[state=on]:text-text data-[state=on]:shadow-[0_0_0_1px_var(--border),0_1px_2px_rgb(0_0_0/0.06)]`}
        >
          {o.label}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}
