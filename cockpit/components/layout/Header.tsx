interface HeaderProps {
  title: string;
  subtitle?: string;
}

export function Header({ title, subtitle }: HeaderProps) {
  return (
    <header className="border-b border-border px-4 py-2 flex items-baseline gap-3">
      <h1 className="text-sm text-text">{title}</h1>
      {subtitle && <span className="text-xs text-text-muted">{subtitle}</span>}
    </header>
  );
}
