import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { usePasswordResetRequest, usePasswordResetConfirm } from "@/hooks/queries";
import { FormShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";

export function ForgotPasswordPage() {
  const navigate = useNavigate();
  const requestReset = usePasswordResetRequest();
  const confirmReset = usePasswordResetConfirm();

  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [awaitingCode, setAwaitingCode] = useState(false);
  const [info, setInfo] = useState("");
  const [error, setError] = useState("");

  const onRequest = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      await requestReset.mutateAsync({ email });
      setAwaitingCode(true);
      setInfo("Se esse email estiver cadastrado, enviamos um código de 6 dígitos.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao enviar o código.");
    }
  };

  const onConfirm = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    if (password !== confirmPassword) return setError("As senhas não coincidem.");
    if (password.length < 8) return setError("A senha deve ter pelo menos 8 caracteres.");
    try {
      await confirmReset.mutateAsync({ email, code, newPassword: password });
      navigate("/login");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Não foi possível redefinir a senha.");
    }
  };

  return (
    <FormShell
      title="Recuperar senha"
      subtitle={awaitingCode ? info : "Informe seu email para receber um código de redefinição."}
    >
      {awaitingCode ? (
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
          <Input
            type="password"
            placeholder="Nova senha"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          <Input
            type="password"
            placeholder="Confirmar nova senha"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
          />
          <Button type="submit" disabled={confirmReset.isPending}>
            {confirmReset.isPending ? "Redefinindo..." : "Redefinir senha"}
          </Button>
        </form>
      ) : (
        <form onSubmit={onRequest} className="flex flex-col gap-4">
          {error && <ErrorBanner>{error}</ErrorBanner>}
          <Input
            type="email"
            placeholder="Email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
          <Button type="submit" disabled={requestReset.isPending}>
            {requestReset.isPending ? "Enviando código..." : "Enviar código"}
          </Button>
        </form>
      )}
      <p className="mt-4 text-sm text-ink-muted">
        Lembrou a senha?{" "}
        <Link to="/login" className="font-semibold text-mint-dark hover:underline">
          Voltar ao login
        </Link>
      </p>
    </FormShell>
  );
}
