import { useState, type FormEvent } from "react";
import { Link, Navigate, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { useLogin } from "@/hooks/queries";
import { FormShell } from "@/components/PageShell";
import { AuthPendingCard } from "@/components/AuthPendingCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";

export function LoginPage() {
  const { user, loading, applySession } = useAuth();
  const navigate = useNavigate();
  const login = useLogin();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  if (loading) return <AuthPendingCard message="Verificando sua sessão no Presumidos..." />;
  if (user) return <Navigate to="/" replace />;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      const result = await login.mutateAsync({ username, password });
      await applySession(result);
      navigate("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao entrar.");
    }
  };

  return (
    <FormShell title="Entrar" subtitle="Acesse sua conta para ver seus bolões.">
      <form onSubmit={onSubmit} className="flex flex-col gap-4">
        {error && <ErrorBanner>{error}</ErrorBanner>}
        <Input
          type="text"
          placeholder="Usuário ou email"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          required
        />
        <Input
          type="password"
          placeholder="Senha"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
        />
        <Button type="submit" disabled={login.isPending}>
          {login.isPending ? "Entrando..." : "Entrar"}
        </Button>
      </form>
      <p className="mt-4 text-sm text-ink-muted">
        Não tem conta?{" "}
        <Link to="/register" className="font-semibold text-mint-dark hover:underline">
          Registre-se aqui
        </Link>
      </p>
      <p className="mt-1 text-sm text-ink-muted">
        <Link to="/forgot-password" className="font-semibold text-mint-dark hover:underline">
          Esqueci minha senha
        </Link>
      </p>
    </FormShell>
  );
}
