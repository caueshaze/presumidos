import { useEffect, useState, type FormEvent } from "react";
import { useAuth } from "@/hooks/useAuth";
import { useChangeUsername } from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label, ErrorBanner } from "@/components/ui/field";

export function ContaPage() {
  const { user } = useAuth();
  const changeUsername = useChangeUsername();

  const [username, setUsername] = useState(user?.username ?? "");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  // Mantém o campo sincronizado com a sessão (ex.: após salvar/recarregar).
  useEffect(() => {
    if (user?.username) setUsername(user.username);
  }, [user?.username]);

  // A confirmação some sozinha depois de alguns segundos.
  useEffect(() => {
    if (!saved) return;
    const timer = setTimeout(() => setSaved(false), 4000);
    return () => clearTimeout(timer);
  }, [saved]);

  const trimmed = username.trim();
  const unchanged = trimmed === (user?.username ?? "");

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    setSaved(false);
    try {
      await changeUsername.mutateAsync(trimmed);
      setSaved(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao alterar o nome.");
    }
  };

  return (
    <PageShell>
      <h1 className="text-3xl">Sua conta</h1>
      <p className="mt-2 max-w-3xl text-sm text-ink-muted">
        Atualize seu nome de usuário. Ele aparece para os outros participantes nos bolões e no
        ranking.
      </p>

      <Card className="mt-6 max-w-md">
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <div>
            <Label>E-mail</Label>
            <Input value={user?.email ?? ""} disabled readOnly />
          </div>

          <div>
            <Label htmlFor="username">Nome de usuário</Label>
            <Input
              id="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              minLength={3}
              maxLength={32}
              required
              autoComplete="username"
            />
            <p className="mt-1 text-xs text-ink-muted">Entre 3 e 32 caracteres.</p>
          </div>

          {error && <ErrorBanner>{error}</ErrorBanner>}
          {saved && (
            <div className="rounded-md border border-success/40 bg-mint/30 px-4 py-2.5 font-heading font-semibold text-mint-dark">
              Nome atualizado!
            </div>
          )}

          <Button
            type="submit"
            disabled={changeUsername.isPending || unchanged || trimmed.length < 3}
            className="self-start"
          >
            {changeUsername.isPending ? "Salvando..." : "Salvar nome"}
          </Button>
        </form>
      </Card>
    </PageShell>
  );
}
