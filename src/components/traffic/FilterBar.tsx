import { useStore, type FilterMethod, type FilterScheme, type FilterStatus } from "../../store";

const METHODS: { value: FilterMethod; label: string }[] = [
  { value: "ALL", label: "All" },
  { value: "GET", label: "GET" },
  { value: "POST", label: "POST" },
  { value: "PUT", label: "PUT" },
  { value: "DELETE", label: "DELETE" },
  { value: "PATCH", label: "PATCH" },
  { value: "HEAD", label: "HEAD" },
  { value: "OPTIONS", label: "OPTIONS" },
  { value: "CONNECT", label: "CONNECT" },
];

const SCHEMES: { value: FilterScheme; label: string }[] = [
  { value: "ALL", label: "All" },
  { value: "Http", label: "HTTP" },
  { value: "Https", label: "HTTPS" },
];

const STATUSES: { value: FilterStatus; label: string }[] = [
  { value: "ALL", label: "All" },
  { value: "2xx", label: "2xx" },
  { value: "3xx", label: "3xx" },
  { value: "4xx", label: "4xx" },
  { value: "5xx", label: "5xx" },
];

export function FilterBar() {
  const filter = useStore((s) => s.filter);
  const setFilter = useStore((s) => s.setFilter);
  const resetFilter = useStore((s) => s.resetFilter);
  const total = useStore((s) => s.sessionList.length);
  const filtered = useStore((s) => s.filteredSessionList.length);
  const hasFilter = filter.searchText || filter.method !== "ALL" || filter.scheme !== "ALL" || filter.status !== "ALL";

  return (
    <div className="flex items-center gap-2 px-4 py-1.5 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
      <div className="relative flex-1 max-w-xs">
        <input
          type="text"
          value={filter.searchText}
          onChange={(e) => setFilter({ searchText: e.target.value })}
          placeholder="Filter by URL, host, method..."
          className="w-full bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-3 py-1.5 pl-7 rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none placeholder:text-[var(--color-text-secondary)]"
        />
        <svg className="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-[var(--color-text-secondary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
      </div>

      <select
        value={filter.method}
        onChange={(e) => setFilter({ method: e.target.value as FilterMethod })}
        className="bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-2 py-1.5 rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer"
      >
        {METHODS.map((m) => (
          <option key={m.value} value={m.value}>{m.label}</option>
        ))}
      </select>

      <select
        value={filter.scheme}
        onChange={(e) => setFilter({ scheme: e.target.value as FilterScheme })}
        className="bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-2 py-1.5 rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer"
      >
        {SCHEMES.map((s) => (
          <option key={s.value} value={s.value}>{s.label}</option>
        ))}
      </select>

      <select
        value={filter.status}
        onChange={(e) => setFilter({ status: e.target.value as FilterStatus })}
        className="bg-[var(--color-bg-tertiary)] text-[var(--color-text-primary)] text-xs px-2 py-1.5 rounded border border-[var(--color-border)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer"
      >
        {STATUSES.map((s) => (
          <option key={s.value} value={s.value}>{s.label}</option>
        ))}
      </select>

      {hasFilter && (
        <button
          onClick={resetFilter}
          className="text-xs text-[var(--color-accent)] hover:underline"
        >
          Reset
        </button>
      )}

      {hasFilter && (
        <span className="text-xs text-[var(--color-text-secondary)]">
          {filtered}/{total}
        </span>
      )}
    </div>
  );
}
