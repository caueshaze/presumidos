import { useEffect, useState, type FormEvent } from "react";
import { Bell, BellOff, Smartphone } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { useChangeUsername } from "@/hooks/queries";
import { usePushReminders } from "@/hooks/usePushReminders";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label, ErrorBanner } from "@/components/ui/field";

export function ContaPage() {
  const { user } = useAuth();
  const changeUsername = useChangeUsername();
  const pushReminders = usePushReminders();

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
  const selectedLeadTime = pushReminders.preference.leadTimeMinutes;

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

      <Card className="mt-6 max-w-3xl">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold uppercase tracking-[0.18em] text-mint-dark">
              <Bell className="h-4 w-4" />
              Notificações
            </div>
            <h2 className="mt-2 text-2xl">Lembretes de palpite</h2>
            <p className="mt-2 max-w-2xl text-sm text-ink-muted">
              Quando ativados, os lembretes avisam neste navegador quando estiver perto do jogo e
              ainda faltar palpite. O tempo vale para a conta inteira; a inscrição do navegador é por dispositivo.
            </p>
          </div>
          <div className="rounded-md border border-mint/25 bg-mint/10 px-4 py-3 text-sm">
            <p className="font-semibold text-ink">
              Dispositivos ativos: {pushReminders.status.data?.activeSubscriptionCount ?? 0}
            </p>
            <p className="mt-1 text-ink-muted">
              Este navegador: {pushReminders.currentDeviceSubscribed ? "conectado" : "não conectado"}
            </p>
          </div>
        </div>

        <div className="mt-6 flex flex-col gap-5">
          <div>
            <Label>Tempo do lembrete</Label>
            <div className="flex flex-wrap gap-2">
              {pushReminders.reminderPresets.map((minutes) => {
                const active = selectedLeadTime === minutes;
                return (
                  <Button
                    key={minutes}
                    type="button"
                    variant={active ? "primary" : "outline"}
                    className="min-w-20"
                    disabled={pushReminders.actionPending}
                    onClick={() =>
                      void pushReminders.updateAccountPreference({
                        enabled: pushReminders.preference.enabled,
                        leadTimeMinutes: minutes,
                      })
                    }
                  >
                    {minutes} min
                  </Button>
                );
              })}
            </div>
            <p className="mt-2 text-xs text-ink-muted">
              O worker manda no máximo um lembrete por jogo para a sua conta.
            </p>
          </div>

          <div className="flex flex-wrap gap-3">
            <Button
              type="button"
              disabled={pushReminders.actionPending || !pushReminders.status.data}
              onClick={() =>
                void pushReminders.enableForCurrentDevice(pushReminders.preference.leadTimeMinutes)
              }
            >
              {pushReminders.preference.enabled && pushReminders.currentDeviceSubscribed
                ? "Reconectar este navegador"
                : "Ativar neste navegador"}
            </Button>

            <Button
              type="button"
              variant="outline"
              disabled={pushReminders.actionPending || !pushReminders.currentDeviceSubscribed}
              onClick={() => void pushReminders.disableCurrentDevice()}
            >
              <BellOff className="mr-2 h-4 w-4" />
              Desativar neste navegador
            </Button>

            <Button
              type="button"
              variant="secondary"
              disabled={
                pushReminders.actionPending ||
                !pushReminders.status.data ||
                pushReminders.preference.enabled
              }
              onClick={() =>
                void pushReminders.updateAccountPreference({
                  enabled: true,
                  leadTimeMinutes: pushReminders.preference.leadTimeMinutes,
                })
              }
            >
              Ligar preferência da conta
            </Button>

            <Button
              type="button"
              variant="outline"
              disabled={
                pushReminders.actionPending ||
                !pushReminders.status.data ||
                !pushReminders.preference.enabled
              }
              onClick={() =>
                void pushReminders.updateAccountPreference({
                  enabled: false,
                  leadTimeMinutes: pushReminders.preference.leadTimeMinutes,
                })
              }
            >
              Desligar preferência da conta
            </Button>
          </div>

          <div className="rounded-md border border-mint/25 bg-white px-4 py-4 text-sm text-ink">
            <p className="font-semibold">
              Status da conta: {pushReminders.preference.enabled ? "ativado" : "desativado"}
            </p>
            <p className="mt-1 text-ink-muted">
              Permissão do navegador:{" "}
              {pushReminders.browserState.permission === "unsupported"
                ? "não suportado"
                : pushReminders.browserState.permission}
            </p>
            {pushReminders.browserState.isProbablyIosBrowser && (
              <div className="mt-3 flex items-start gap-2 rounded-md bg-sky/10 px-3 py-3">
                <Smartphone className="mt-0.5 h-4 w-4 shrink-0 text-sky-dark" />
                <p className="text-ink-muted">
                  No iPhone/iPad, o fluxo completo de push exige abrir o app a partir da Tela
                  Inicial.
                </p>
              </div>
            )}
          </div>

          {pushReminders.actionError && <ErrorBanner>{pushReminders.actionError}</ErrorBanner>}
          {pushReminders.actionMessage && (
            <div className="rounded-md border border-success/40 bg-mint/30 px-4 py-2.5 font-heading font-semibold text-mint-dark">
              {pushReminders.actionMessage}
            </div>
          )}
        </div>
      </Card>
    </PageShell>
  );
}
