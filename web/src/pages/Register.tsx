import { useState, type FormEvent } from "react";
import { Link, Navigate, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { useRegisterRequest, useRegisterConfirm } from "@/hooks/queries";
import { FormShell } from "@/components/PageShell";
import { AuthPendingCard } from "@/components/AuthPendingCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";

export function RegisterPage() {
  const { user, loading, applySession } = useAuth();
  const navigate = useNavigate();
  const requestReg = useRegisterRequest();
  const confirmReg = useRegisterConfirm();

  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [code, setCode] = useState("");
  const [awaitingCode, setAwaitingCode] = useState(false);
  const [error, setError] = useState("");

  if (loading) return <AuthPendingCard message="Verificando sua sessão no Presumidos..." />;
  if (user) return <Navigate to="/" replace />;

  const onRequest = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    if (password !== confirmPassword) return setError("As senhas não coincidem.");
    if (password.length < 8) return setError("A senha deve ter pelo menos 8 caracteres.");
    try {
      await requestReg.mutateAsync({ username, email, password });
      setAwaitingCode(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao enviar o código.");
    }
  };

  const onConfirm = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      const result = await confirmReg.mutateAsync({ email, code });
      await applySession(result);
      navigate("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Código inválido.");
    }
  };

  if (awaitingCode) {
    return (
      <FormShell
        title="Criar conta"
        subtitle={`Enviamos um código de 6 dígitos para ${email}. Digite-o abaixo para confirmar.`}
      >
        <form onSubmit={onConfirm} className="flex flex-col gap-4">
          {error && <ErrorBanner>{error}</ErrorBanner>}
          <Input
            type="text"
            inputMode="numeric"
            maxLength={6}
            placeholder="Código de 6 dígitos"
            value={code}
            onChange={(e) => setCode(e.target.value)}
            required
          />
          <Button type="submit" disabled={confirmReg.isPending}>
            {confirmReg.isPending ? "Confirmando..." : "Confirmar conta"}
          </Button>
        </form>
        <button
          type="button"
          className="mt-4 text-sm font-semibold text-mint-dark hover:underline"
          onClick={() => {
            setAwaitingCode(false);
            setCode("");
            setError("");
          }}
        >
          Voltar e corrigir os dados
        </button>
      </FormShell>
    );
  }

  return (
    <FormShell title="Criar conta" subtitle="Cadastre-se para criar ou entrar em bolões.">
      <form onSubmit={onRequest} className="flex flex-col gap-4">
        {error && <ErrorBanner>{error}</ErrorBanner>}
        <Input
          type="text"
          placeholder="Nome de usuário"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          required
        />
        <Input
          type="email"
          placeholder="Email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          required
        />
        <Input
          type="password"
          placeholder="Senha"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
        />
        <Input
          type="password"
          placeholder="Confirmar senha"
          value={confirmPassword}
          onChange={(e) => setConfirmPassword(e.target.value)}
          required
        />
        <Button type="submit" disabled={requestReg.isPending}>
          {requestReg.isPending ? "Enviando código..." : "Criar conta"}
        </Button>
      </form>
      <p className="mt-4 text-sm text-ink-muted">
        Já tem conta?{" "}
        <Link to="/login" className="font-semibold text-mint-dark hover:underline">
          Faça login aqui
        </Link>
      </p>
      <p className="mt-4 text-xs leading-5 text-ink-muted">
        Ao continuar, você concorda com os{" "}
        <Link to="/terms" className="font-semibold text-mint-dark hover:underline">
          Termos de Uso
        </Link>{" "}
        e a{" "}
        <Link to="/privacy" className="font-semibold text-mint-dark hover:underline">
          Política de Privacidade
        </Link>
        .
      </p>
    </FormShell>
  );
}
