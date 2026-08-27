import { Navigate, useLocation } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { AuthPendingCard } from "./AuthPendingCard";

/** Protege rotas autenticadas: aguarda a sessão e redireciona ao login se ausente. */
export function AuthGuard({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  const location = useLocation();

  if (loading) {
    return <AuthPendingCard message="Verificando sua sessão no Presumidos..." />;
  }
  if (!user) {
    const from = `${location.pathname}${location.search}${location.hash}`;
    const legacyInvite = location.pathname === "/dashboard"
      ? new URLSearchParams(location.search).get("invite")
      : null;
    const returnTo = legacyInvite
      ? `/pools/join/${encodeURIComponent(legacyInvite)}`
      : from;
    return <Navigate to={`/login?returnTo=${encodeURIComponent(returnTo)}`} replace />;
  }
  return <>{children}</>;
}
