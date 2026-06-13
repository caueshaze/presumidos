import { ApiError } from "./api";

/**
 * Executa uma ação de admin. Se o backend exigir reautenticação
 * (403 SECURITY:ADMIN_REAUTH_REQUIRED), pede a senha, reautentica e tenta de novo.
 */
export async function withAdminReauth<T>(
  action: () => Promise<T>,
  reauth: (password: string) => Promise<void>,
): Promise<T> {
  try {
    return await action();
  } catch (err) {
    if (err instanceof ApiError && err.needsAdminReauth) {
      const password = window
        .prompt("Confirme sua senha de administrador para continuar")
        ?.trim();
      if (!password) throw new Error("Confirmação de administrador cancelada.");
      await reauth(password);
      return await action();
    }
    throw err;
  }
}
