export function HeadersTable({ headers }: { headers: [string, string][] }) {
  if (headers.length === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic">无请求头</div>;
  }
  return (
    <table className="w-full text-xs">
      <tbody>
        {headers.map(([key, value], i) => (
          <tr key={i} className="border-b border-[var(--color-border)]">
            <td className="py-1 pr-4 text-[var(--color-accent)] font-mono whitespace-nowrap align-top">
              {key}
            </td>
            <td className="py-1 text-[var(--color-text-primary)] break-all">
              {value}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
