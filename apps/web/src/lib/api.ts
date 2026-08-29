// Cliente HTTP da API Presumidos.
//
// - Cookie de sessão é HttpOnly (setado pelo backend) → sempre `credentials: "include"`.
// - Mutações exigem o header `X-CSRF-Token`. O token vem da sessão; mantemos um cache em
//   memória, atualizado a partir das respostas de auth e, se faltar, buscado em /api/auth/csrf.

const API_BASE = "/api";

export class ApiError extends Error {
  status: number;
  errorId: string | null;
  /** true quando o backend pede reautenticação de admin (403 SECURITY:ADMIN_REAUTH_REQUIRED). */
  needsAdminReauth: boolean;

  constructor(status: number, message: string, errorId: string | null = null) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.errorId = errorId;
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
    let errorId: string | null = null;
    try {
      const data = await res.json();
      if (data && typeof data.error === "string") message = data.error;
      if (data && typeof data.errorId === "string") errorId = data.errorId;
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

    if (res.status >= 500 && errorId) {
      message = `Não foi possível concluir a operação. Código do erro: ${errorId}`;
    }
    throw new ApiError(res.status, message, errorId);
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
  upload: async <T>(path: string, file: File, fields: Record<string, string> = {}): Promise<T> => {
    const form = new FormData();
    form.append("file", file);
    Object.entries(fields).forEach(([key, value]) => form.append(key, value));
    const send = async () => fetch(`${API_BASE}${path}`, {
      method: "POST",
      credentials: "include",
      headers: { "X-CSRF-Token": await ensureCsrfToken() },
      body: form,
    });
    let res = await send();
    if (res.status === 403) {
      let message = "";
      try { message = ((await res.clone().json()) as { error?: string }).error ?? ""; } catch { /* resposta não JSON */ }
      if (message !== "SECURITY:ADMIN_REAUTH_REQUIRED") {
        setCsrfToken(null);
        await fetchCsrfToken();
        res = await send();
      }
    }
    if (!res.ok) {
      let message = res.statusText;
      let errorId: string | null = null;
      try {
        const data = (await res.json()) as { error?: string; errorId?: string };
        message = data.error ?? message;
        errorId = data.errorId ?? null;
      } catch { /* resposta sem corpo */ }
      if (res.status >= 500 && errorId) {
        message = `Não foi possível concluir a operação. Código do erro: ${errorId}`;
      }
      throw new ApiError(res.status, message, errorId);
    }
    return (await res.json()) as T;
  },
  download: async (path: string): Promise<{ blob: Blob; filename: string | null }> => {
    const res = await fetch(`${API_BASE}${path}`, {
      credentials: "include",
    });
    if (!res.ok) {
      let message = res.statusText;
      let errorId: string | null = null;
      try {
        const data = await res.json();
        if (data && typeof data.error === "string") message = data.error;
        if (data && typeof data.errorId === "string") errorId = data.errorId;
      } catch {
        // resposta sem corpo JSON
      }
      if (res.status >= 500 && errorId) {
        message = `Não foi possível concluir a operação. Código do erro: ${errorId}`;
      }
      throw new ApiError(res.status, message, errorId);
    }
    const disposition = res.headers.get("content-disposition");
    const filename = disposition?.match(/filename="([^"]+)"/)?.[1] ?? null;
    return { blob: await res.blob(), filename };
  },
};
