import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Bell, CheckCircle2, Circle, Smartphone, X } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { usePools, useMatches, useMyPredictions, useLeaderboard } from "@/hooks/queries";
import { usePushReminders } from "@/hooks/usePushReminders";
import { formatSelectionLabel } from "@/lib/selections";
import { formatKickoff } from "@/lib/utils";
import type { UserPublic } from "@/types";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

const benefits = [
  {
    icon: "⚔️",
    title: "Competição boa de verdade",
    text: "Cada rodada vira uma disputa divertida entre amigos, sem complicação para entrar e começar.",
  },
  {
    icon: "📈",
    title: "Ranking sempre à vista",
    text: "Acompanhe quem está dominando o bolão com uma leitura rápida do pódio e da tabela.",
  },
  {
    icon: "🎯",
    title: "Palpite rápido e direto",
    text: "Salve seus placares com poucos cliques e foque no jogo, não no formulário.",
  },
];

export function HomePage() {
  const { user } = useAuth();
  return user ? <LoggedInHome user={user} /> : <MarketingHome />;
}

// ---------------------------------------------------------------------------
// Home logado: resumo operacional
// ---------------------------------------------------------------------------

const section = {
  initial: { opacity: 0, y: 16 },
  animate: { opacity: 1, y: 0 },
};

function LoggedInHome({ user }: { user: UserPublic }) {
  const navigate = useNavigate();
  const [reminderBannerDismissed, setReminderBannerDismissed] = useState(false);
  const pools = usePools();
  const matches = useMatches();
  const predictions = useMyPredictions();
  const pushReminders = usePushReminders();
  const firstPoolId = pools.data?.[0]?.id ?? null;
  const leaderboard = useLeaderboard(firstPoolId);

  const now = Date.now();
  const upcoming = (matches.data ?? [])
    .filter((m) => new Date(m.kickoff).getTime() > now)
    .slice(0, 5);
  const nextGame = upcoming[0];
  const predictedIds = new Set((predictions.data ?? []).map((p) => p.matchId));

  const ranking = leaderboard.data ?? [];
  const hasAnyPoints = ranking.some((entry) => entry.points > 0);
  const myIndex = ranking.findIndex((e) => e.username === user.username);
  const myEntry = myIndex >= 0 ? ranking[myIndex] : null;
  const firstPool = pools.data?.[0];
  const reminderBannerKey = `presumidos:push-reminder-banner:dismissed:${user.id}`;

  useEffect(() => {
    if (typeof window === "undefined") return;
    setReminderBannerDismissed(window.localStorage.getItem(reminderBannerKey) === "1");
  }, [reminderBannerKey]);

  const showReminderBanner =
    !reminderBannerDismissed &&
    !pushReminders.status.isLoading &&
    (!pushReminders.preference.enabled ||
      !pushReminders.currentDeviceSubscribed ||
      pushReminders.browserState.permission !== "granted");

  const enableReminders = async () => {
    await pushReminders.enableForCurrentDevice(pushReminders.preference.leadTimeMinutes);
  };

  const dismissReminderBanner = () => {
    setReminderBannerDismissed(true);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(reminderBannerKey, "1");
    }
  };

  return (
    <PageShell>
      <h1 className="text-3xl sm:text-4xl">Olá, {user.username} 👋</h1>

      <div className="mt-5 flex flex-wrap gap-3">
        <Button onClick={() => navigate("/dashboard")}>Ver meus bolões</Button>
        <Button variant="secondary" onClick={() => navigate("/predictions")}>
          Palpitar próximos jogos
        </Button>
        <Button variant="outline" onClick={() => navigate("/leaderboard")}>
          Ver ranking
        </Button>
      </div>

      {showReminderBanner && (
        <motion.section {...section} transition={{ duration: 0.3 }} className="mt-8">
          <Card className="border border-sky/20 bg-white/70 p-4">
            <div className="flex flex-col gap-4">
              <div className="flex items-start justify-between gap-3">
                <div className="max-w-3xl">
                  <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-sky-dark">
                    <Bell className="h-4 w-4" />
                    Lembretes de palpite
                  </div>
                  <h2 className="mt-1 text-lg">Receba aviso antes do jogo começar</h2>
                  <p className="mt-1 text-sm text-ink-muted">
                    Ative notificações neste navegador para não esquecer jogo aberto sem palpite.
                    Você pode ajustar isso depois em Conta.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={dismissReminderBanner}
                  aria-label="Fechar lembrete de notificações"
                  className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-ink-muted transition-colors hover:bg-mint/10 hover:text-ink"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>

              {pushReminders.browserState.isProbablyIosBrowser && (
                <div className="rounded-md border border-mint/20 bg-bg px-4 py-3 text-sm text-ink">
                  <div className="flex items-center gap-2 font-semibold">
                    <Bell className="h-4 w-4" />
                    <Smartphone className="h-4 w-4 text-mint-dark" />
                    iPhone/iPad
                  </div>
                  <p className="mt-1 text-ink-muted">
                    No iPhone/iPad, as notificações web pedem que o app seja adicionado à Tela
                    Inicial antes de ativar o lembrete.
                  </p>
                </div>
              )}

              {pushReminders.actionError && (
                <p className="text-sm font-semibold text-danger">{pushReminders.actionError}</p>
              )}
              {pushReminders.actionMessage && (
                <p className="text-sm font-semibold text-mint-dark">{pushReminders.actionMessage}</p>
              )}

              <div className="flex shrink-0 flex-wrap gap-2">
                <Button
                  size="sm"
                  onClick={() => void enableReminders()}
                  disabled={pushReminders.actionPending || !pushReminders.status.data}
                >
                  {pushReminders.actionPending ? "Ativando..." : "Ativar"}
                </Button>
                <Button variant="outline" size="sm" onClick={dismissReminderBanner}>
                  Agora não
                </Button>
                <Button variant="secondary" size="sm" onClick={() => navigate("/conta")}>
                  Ajustar em Conta
                </Button>
              </div>
            </div>
          </Card>
        </motion.section>
      )}

      {/* Seus bolões */}
      <motion.section {...section} transition={{ duration: 0.3 }} className="mt-8">
        <h2 className="mb-3 text-xl">Seus bolões</h2>
        {pools.isLoading ? (
          <Card>
            <p className="text-ink-muted">Carregando...</p>
          </Card>
        ) : pools.data && pools.data.length > 0 ? (
          <div className="grid gap-4 sm:grid-cols-2">
            {pools.data.map((p) => (
              <Card
                key={p.id}
                className="cursor-pointer hover:shadow-card-hover"
                onClick={() => navigate("/dashboard")}
              >
                <h3 className="text-lg">{p.name}</h3>
                <p className="mt-1 text-sm text-ink-muted">{p.memberCount} participante(s)</p>
                <p className="mt-2 text-sm">
                  <span className="text-ink-muted">Próximo jogo: </span>
                  {nextGame ? (
                  <span className="font-semibold">
                      {formatSelectionLabel(nextGame.homeTeam)} x {formatSelectionLabel(nextGame.awayTeam)}
                    </span>
                  ) : (
                    <span className="text-ink-muted">sem jogos agendados</span>
                  )}
                </p>
              </Card>
            ))}
          </div>
        ) : (
          <Card>
            <p className="text-ink-muted">Você ainda não participa de nenhum bolão.</p>
            <Button className="mt-3" size="sm" onClick={() => navigate("/dashboard")}>
              Criar ou entrar em um bolão
            </Button>
          </Card>
        )}
      </motion.section>

      {/* Próximos palpites */}
      <motion.section {...section} transition={{ duration: 0.3, delay: 0.05 }} className="mt-8">
        <h2 className="mb-3 text-xl">Próximos palpites</h2>
        <Card className="p-0">
          {matches.isLoading || predictions.isLoading ? (
            <p className="p-6 text-ink-muted">Carregando...</p>
          ) : upcoming.length === 0 ? (
            <p className="p-6 text-ink-muted">Nenhum jogo aberto para palpite no momento.</p>
          ) : (
            <ul className="divide-y divide-mint/20">
              {upcoming.map((game) => {
                const done = predictedIds.has(game.id);
                return (
                  <li
                    key={game.id}
                    className="flex cursor-pointer items-center justify-between gap-3 px-5 py-3 transition-colors hover:bg-mint/10"
                    onClick={() => navigate("/predictions")}
                  >
                    <div>
                      <div className="font-semibold">
                        {formatSelectionLabel(game.homeTeam)} x {formatSelectionLabel(game.awayTeam)}
                      </div>
                      <div className="text-xs text-ink-muted">{formatKickoff(game.kickoff)}</div>
                    </div>
                    {done ? (
                      <span className="inline-flex shrink-0 items-center gap-1 rounded-pill bg-success/15 px-3 py-1 text-xs font-semibold text-mint-dark ring-1 ring-success/40">
                        <CheckCircle2 className="h-3.5 w-3.5" strokeWidth={2.5} />
                        palpite salvo
                      </span>
                    ) : (
                      <span className="inline-flex shrink-0 items-center gap-1 rounded-pill bg-yellow/50 px-3 py-1 text-xs font-semibold text-ink">
                        <Circle className="h-3.5 w-3.5" strokeWidth={2.5} />
                        falta palpite
                      </span>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </Card>
      </motion.section>

      {/* Ranking geral */}
      <motion.section {...section} transition={{ duration: 0.3, delay: 0.1 }} className="mt-8">
        <h2 className="mb-3 text-xl">Ranking geral</h2>
        <Card>
          {!firstPool ? (
            <p className="text-ink-muted">Entre em um bolão para disputar o ranking.</p>
          ) : leaderboard.isLoading ? (
            <p className="text-ink-muted">Carregando...</p>
          ) : !hasAnyPoints ? (
            <p className="text-ink-muted">
              Ainda ninguém pontuou no <span className="font-semibold">{firstPool.name}</span>.
            </p>
          ) : myEntry ? (
            <p className="text-lg">
              No <span className="font-semibold">{firstPool.name}</span>, você está em{" "}
              <span className="font-heading text-2xl font-bold text-mint-dark">
                {myIndex + 1}º lugar
              </span>{" "}
              com <span className="font-semibold">{myEntry.points} pontos</span>.
            </p>
          ) : (
            <p className="text-ink-muted">
              Ainda sem pontos no <span className="font-semibold">{firstPool.name}</span>. Os pontos
              entram quando os resultados oficiais saírem.
            </p>
          )}
        </Card>
      </motion.section>
    </PageShell>
  );
}

// ---------------------------------------------------------------------------
// Home pública: marketing
// ---------------------------------------------------------------------------

function MarketingHome() {
  const navigate = useNavigate();
  return (
    <PageShell>
      <section className="mb-10 text-center">
        <span className="font-heading text-sm font-semibold uppercase tracking-widest text-mint-dark">
          Presumidos
        </span>
        <h1 className="mt-2 text-4xl sm:text-5xl">⚽ Presumidos da Copa 2026</h1>
        <p className="mx-auto mt-3 max-w-2xl text-lg text-ink-muted">
          O bolão Presumidos transforma cada palpite em disputa, resenha e ranking entre amigos.
        </p>
        <div className="mt-6 flex flex-wrap justify-center gap-3">
          <Button onClick={() => navigate("/register")}>Criar conta no Presumidos</Button>
          <Button variant="secondary" onClick={() => navigate("/login")}>
            Entrar para acompanhar
          </Button>
        </div>
      </section>

      <Card className="mx-auto mb-8 max-w-2xl">
        <h2 className="text-2xl">Entre, chame a galera e deixe o ranking falar</h2>
        <p className="mt-3 text-ink-muted">
          📝 Cadastre-se, crie seu bolão no Presumidos ou entre em um convite já criado.
        </p>
        <p className="mt-2 text-ink-muted">
          🔮 Salve seus palpites antes do apito inicial e acompanhe tudo sem fricção.
        </p>
        <p className="mt-2 text-ink-muted">
          🏆 Quando os resultados oficiais entram, o ranking se atualiza e a resenha começa.
        </p>
      </Card>

      <div className="grid gap-5 sm:grid-cols-3">
        {benefits.map((b, i) => (
          <motion.div
            key={b.title}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 + i * 0.08, duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
          >
            <Card className="h-full">
              <span className="text-3xl">{b.icon}</span>
              <h3 className="mt-3 text-lg">{b.title}</h3>
              <p className="mt-1 text-sm text-ink-muted">{b.text}</p>
            </Card>
          </motion.div>
        ))}
      </div>
    </PageShell>
  );
}
