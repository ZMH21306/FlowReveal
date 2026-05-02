export function TlsInfoBadge({ tlsInfo }: { tlsInfo: { version: string; cipher_suite: string; server_name: string | null } }) {
  return (
    <div className="flex items-center gap-2 text-xs text-[var(--color-accent)] bg-[var(--color-bg-tertiary)] px-2 py-1 rounded">
      <span>🔓 {tlsInfo.version}</span>
      <span className="text-[var(--color-text-secondary)]">|</span>
      <span>{tlsInfo.cipher_suite}</span>
      {tlsInfo.server_name && (
        <>
          <span className="text-[var(--color-text-secondary)]">|</span>
          <span>SNI: {tlsInfo.server_name}</span>
        </>
      )}
    </div>
  );
}
