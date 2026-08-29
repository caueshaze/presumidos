/**
 * Aceita somente destinos internos absolutos da aplicação.
 * Isso é usado em links de autenticação iniciados por convites e evita
 * transformar `returnTo` em um open redirect.
 */
export function safeReturnTo(value: string | null | undefined, fallback = "/"): string {
  if (!value || !value.startsWith("/") || value.startsWith("//") || value.includes("\\")) {
    return fallback;
  }
  try {
    const parsed = new URL(value, window.location.origin);
    if (parsed.origin !== window.location.origin) return fallback;
    return `${parsed.pathname}${parsed.search}${parsed.hash}`;
  } catch {
    return fallback;
  }
}

export function authReturnTo(path: string): string {
  return `/login?returnTo=${encodeURIComponent(safeReturnTo(path, "/"))}`;
}

export function registerReturnTo(path: string): string {
  return `/register?returnTo=${encodeURIComponent(safeReturnTo(path, "/"))}`;
}
