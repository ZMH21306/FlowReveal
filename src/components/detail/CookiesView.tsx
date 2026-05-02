export function CookiesView({ headers }: { headers: [string, string][] }) {
  const cookies = headers
    .filter(([k]) => k.toLowerCase() === "cookie")
    .flatMap(([, v]) =>
      v.split(";").map((pair) => {
        const [name, ...rest] = pair.trim().split("=");
        return { name: name?.trim() || "", value: rest.join("=").trim() };
      })
    );

  const setCookies = headers
    .filter(([k]) => k.toLowerCase() === "set-cookie")
    .map(([, v]) => {
      const [nameValue, ...attrs] = v.split("; ");
      const [name, ...rest] = nameValue.split("=");
      return {
        name: name?.trim() || "",
        value: rest.join("=").trim(),
        attrs: attrs.join("; "),
      };
    });

  if (cookies.length === 0 && setCookies.length === 0) {
    return <div className="text-xs text-[var(--color-text-secondary)] italic">无 Cookie</div>;
  }

  return (
    <div className="space-y-3">
      {cookies.length > 0 && (
        <div>
          <div className="text-xs font-semibold text-[var(--color-text-secondary)] mb-1">请求 Cookie</div>
          <table className="w-full text-xs">
            <tbody>
              {cookies.map((c, i) => (
                <tr key={i} className="border-b border-[var(--color-border)]">
                  <td className="py-1 pr-4 text-[var(--color-accent)] font-mono">{c.name}</td>
                  <td className="py-1 text-[var(--color-text-primary)] break-all">{c.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {setCookies.length > 0 && (
        <div>
          <div className="text-xs font-semibold text-[var(--color-text-secondary)] mb-1">响应 Set-Cookie</div>
          <table className="w-full text-xs">
            <tbody>
              {setCookies.map((c, i) => (
                <tr key={i} className="border-b border-[var(--color-border)]">
                  <td className="py-1 pr-4 text-[var(--color-accent)] font-mono">{c.name}</td>
                  <td className="py-1 text-[var(--color-text-primary)] break-all">{c.value}</td>
                  <td className="py-1 pl-4 text-[var(--color-text-secondary)] text-[10px]">{c.attrs}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
