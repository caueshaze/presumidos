// Cliente HTTP da API Presumidos.
//
// - Cookie de sessão é HttpOnly (setado pelo backend) → sempre `credentials: "include"`.
// - Mutações exigem o header `X-CSRF-Token`. O token vem da sessão; mantemos um cache em
//   memória, atualizado a partir das respostas de auth e, se faltar, buscado em /api/auth/csrf.

const API_BASE = "/api";

export class ApiError extends Error {
  status: number;
  /** true quando o backend pede reautenticação de admin (403 SECURITY:ADMIN_REAUTH_REQUIRED). */
  needsAdminReauth: boolean;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.needsAdminReauth = message === "SECURITY:ADMIN_REAUTH_REQUIRED";
  }
}

let csrfToken: string | null = null;

export function setCsrfToken(token: string | null) {
  csrfToken = token && token.length > 0 ? token : null;
}

async function fetchCsrfToken(): Promise<string> {
  const res = await fetch(`${API_BASE}/auth/csrf`, { credentials: "include" });
  if (!res.ok) throw new ApiError(res.status, "Não foi possível obter o token de segurança.");
  const data = (await res.json()) as { csrfToken: string };
  setCsrfToken(data.csrfToken);
  return data.csrfToken;
}

async function ensureCsrfToken(): Promise<string> {
  return csrfToken ?? (await fetchCsrfToken());
}

interface RequestOptions {
  method?: "GET" | "POST";
  body?: unknown;
  /** Anexa o header X-CSRF-Token (padrão: true para POST). */
  csrf?: boolean;
  /** Uso interno: evita loop infinito ao reexecutar após renovar o CSRF. */
  _retried?: boolean;
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const method = options.method ?? "GET";
  const needsCsrf = options.csrf ?? method !== "GET";

  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (needsCsrf) headers["X-CSRF-Token"] = await ensureCsrfToken();

  const res = await fetch(`${API_BASE}${path}`, {
    method,
    credentials: "include",
    headers,
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
  });

  if (!res.ok) {
    let message = res.statusText;
    try {
      const data = await res.json();
      if (data && typeof data.error === "string") message = data.error;
    } catch {
      // resposta sem corpo JSON
    }

    // CSRF expirado/dessincronizado (403, mas não a reautenticação de admin):
    // renova o token uma vez e repete a requisição.
    const isAdminReauth = message === "SECURITY:ADMIN_REAUTH_REQUIRED";
    if (res.status === 403 && needsCsrf && !isAdminReauth && !options._retried) {
      setCsrfToken(null);
      await fetchCsrfToken();
      return request<T>(path, { ...options, _retried: true });
    }

    throw new ApiError(res.status, message);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  get: <T>(path: string) => request<T>(path, { method: "GET" }),
  /** POST autenticado: anexa CSRF automaticamente. */
  post: <T>(path: string, body?: unknown) => request<T>(path, { method: "POST", body }),
  /** POST público (login/registro/reset): não exige CSRF. */
  postPublic: <T>(path: string, body?: unknown) =>
    request<T>(path, { method: "POST", body, csrf: false }),
};
