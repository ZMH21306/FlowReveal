export function HeadersTable({ headers }: { headers: [string, string][] }) {
  if (headers.length === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic py-2">无请求头</div>;
  }
  return (
    <table className="w-full text-xs">
      <tbody>
        {headers.map(([key, value], i) => (
          <tr key={i} className="border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)]">
            <td className="py-[6px] pr-4 text-[var(--color-accent)] font-mono whitespace-nowrap align-top w-[140px]">
              {key}
            </td>
            <td className="py-[6px] text-[var(--color-text-primary)] break-all leading-relaxed">
              {value || <span className="italic text-[var(--color-text-secondary)]">(空)</span>}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
