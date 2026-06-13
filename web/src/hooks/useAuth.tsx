import { createContext, useContext, useCallback, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, setCsrfToken } from "@/lib/api";
import type { AuthResult, SessionState, UserPublic } from "@/types";

interface AuthContextValue {
  user: UserPublic | null;
  csrfToken: string;
  isAdmin: boolean;
  loading: boolean;
  /** Aplica a sessão retornada por login/registro e refaz a query de sessão. */
  applySession: (result: AuthResult) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

async function fetchSession(): Promise<SessionState> {
  const session = await api.get<SessionState>("/auth/current-user");
  setCsrfToken(session.csrfToken);
  return session;
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["current-user"],
    queryFn: fetchSession,
    staleTime: 30_000,
    retry: false,
  });

  const applySession = useCallback(
    async (result: AuthResult) => {
      setCsrfToken(result.csrfToken);
      queryClient.setQueryData<SessionState>(["current-user"], {
        user: result.user,
        csrfToken: result.csrfToken,
      });
      await queryClient.invalidateQueries({ queryKey: ["current-user"] });
    },
    [queryClient],
  );

  const logout = useCallback(async () => {
    try {
      await api.post("/auth/logout");
    } finally {
      setCsrfToken(null);
      queryClient.setQueryData<SessionState>(["current-user"], { user: null, csrfToken: "" });
      queryClient.clear();
    }
  }, [queryClient]);

  const value: AuthContextValue = {
    user: data?.user ?? null,
    csrfToken: data?.csrfToken ?? "",
    isAdmin: data?.user?.isAdmin ?? false,
    loading: isLoading,
    applySession,
    logout,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth deve ser usado dentro de <AuthProvider>");
  return ctx;
}
