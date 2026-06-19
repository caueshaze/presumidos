import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { AnimatePresence, motion } from "framer-motion";
import { Plus, X, Trash2, Ticket } from "lucide-react";
import { usePools, useCreatePool, useJoinPool, useDeletePool } from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Card, MotionCard } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";
import { cn } from "@/lib/utils";

type Mode = "create" | "join" | null;

export function DashboardPage() {
  const navigate = useNavigate();
  const { user, isAdmin } = useAuth();
  const pools = usePools();
  const createPool = useCreatePool();
  const joinPool = useJoinPool();
  const deletePool = useDeletePool();

  const onDelete = async (poolId: string, name: string) => {
    if (!window.confirm(`Apagar o bolão "${name}"? Esta ação não pode ser desfeita.`)) return;
    setError("");
    try {
      await deletePool.mutateAsync(poolId);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao apagar bolão.");
    }
  };

  const [mode, setMode] = useState<Mode>(null);
  const [newPoolName, setNewPoolName] = useState("");
  const [joinCode, setJoinCode] = useState("");
  const [error, setError] = useState("");

  // Alterna o painel: clicar no mesmo botão fecha; clicar no outro troca.
  const openMode = (next: Exclude<Mode, null>) => {
    setError("");
    setMode((current) => (current === next ? null : next));
  };

  const onCreate = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      await createPool.mutateAsync(newPoolName);
      setNewPoolName("");
      setMode(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao criar bolão.");
    }
  };

  const onJoin = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      await joinPool.mutateAsync(joinCode);
      setJoinCode("");
      setMode(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao entrar no bolão.");
    }
  };

  return (
    <PageShell>
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <h1 className="text-3xl">Seus bolões</h1>
        <div className="flex flex-col gap-2 sm:flex-row">
          <Button
            variant="primary"
            size="sm"
            aria-pressed={mode === "create"}
            onClick={() => openMode("create")}
            className={cn(
              "justify-center",
              mode === "create" && "ring-2 ring-mint-dark/35 ring-offset-2 ring-offset-bg",
            )}
          >
            <Plus className="h-4 w-4" /> Criar bolão
          </Button>
          <Button
            variant="secondary"
            size="sm"
            aria-pressed={mode === "join"}
            onClick={() => openMode("join")}
            className={cn(
              "justify-center",
              mode === "join" && "ring-2 ring-sky-dark/45 ring-offset-2 ring-offset-bg",
            )}
          >
            <Ticket className="h-4 w-4" /> Entrar com código
          </Button>
        </div>
      </div>

      {/* Um painel focado por vez: a intenção é escolhida no botão, depois um único campo. */}
      <AnimatePresence initial={false} mode="wait">
        {mode === "create" && (
          <motion.div
            key="create"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ height: { duration: 0.26, ease: [0.22, 1, 0.36, 1] }, opacity: { duration: 0.18 } }}
            className="overflow-hidden"
          >
            <Card className="mt-5 border border-mint/30">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="text-xl">Criar um bolão</h2>
                  <p className="mt-1 text-sm text-ink-muted">
                    Você vira o dono e convida a galera com o código que o app gera.
                  </p>
                </div>
                <CloseButton onClick={() => setMode(null)} />
              </div>
              <form onSubmit={onCreate} className="mt-4 flex flex-col gap-3 sm:flex-row">
                <Input
                  className="flex-1"
                  placeholder="Nome do bolão (ex: Bolão da firma)"
                  value={newPoolName}
                  onChange={(e) => setNewPoolName(e.target.value)}
                  autoFocus
                  required
                />
                <Button type="submit" disabled={createPool.isPending} className="sm:self-stretch">
                  {createPool.isPending ? "Criando..." : "Criar bolão"}
                </Button>
              </form>
            </Card>
          </motion.div>
        )}

        {mode === "join" && (
          <motion.div
            key="join"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ height: { duration: 0.26, ease: [0.22, 1, 0.36, 1] }, opacity: { duration: 0.18 } }}
            className="overflow-hidden"
          >
            <Card className="mt-5 border border-sky/30">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="text-xl">Entrar com um código</h2>
                  <p className="mt-1 text-sm text-ink-muted">
                    Recebeu um convite? Digite o código de 6 caracteres pra entrar no bolão.
                  </p>
                </div>
                <CloseButton onClick={() => setMode(null)} />
              </div>
              <form onSubmit={onJoin} className="mt-4 flex flex-col gap-3">
                {/* Campo "catraca": o código é lido e digitado caractere a caractere. */}
                <Input
                  className="text-center font-heading text-2xl font-semibold uppercase tracking-[0.4em] placeholder:font-body placeholder:text-base placeholder:font-normal placeholder:normal-case placeholder:tracking-normal"
                  placeholder="Ex: 3F9A2C"
                  value={joinCode}
                  onChange={(e) => setJoinCode(e.target.value.toUpperCase().slice(0, 12))}
                  autoCapitalize="characters"
                  autoComplete="off"
                  spellCheck={false}
                  autoFocus
                  required
                />
                <Button type="submit" variant="secondary" disabled={joinPool.isPending}>
                  {joinPool.isPending ? "Entrando..." : "Entrar no bolão"}
                </Button>
              </form>
            </Card>
          </motion.div>
        )}
      </AnimatePresence>

      {error && (
        <div className="mt-4">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}

      <div className="mt-6">
        {pools.isLoading ? (
          <Card>
            <p className="text-ink-muted">Carregando...</p>
          </Card>
        ) : pools.isError ? (
          <ErrorBanner>Erro ao carregar bolões: {(pools.error as Error).message}</ErrorBanner>
        ) : pools.data && pools.data.length === 0 ? (
          <Card>
            <h2 className="text-xl">Sua presunção começa aqui!</h2>
            <p className="mt-2 text-ink-muted">
              Você ainda não participa de nenhum bolão no Presumidos.
            </p>
            <p className="mt-1 text-ink-muted">
              Crie um bolão pra reunir a galera, ou entre com o código que te mandaram.
            </p>
            <div className="mt-4 flex flex-col gap-2 sm:flex-row">
              <Button onClick={() => openMode("create")}>
                <Plus className="h-4 w-4" /> Criar um bolão
              </Button>
              <Button variant="secondary" onClick={() => openMode("join")}>
                <Ticket className="h-4 w-4" /> Entrar com código
              </Button>
            </div>
          </Card>
        ) : (
          <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {pools.data?.map((p, i) => (
              <MotionCard key={p.id} transition={{ delay: i * 0.06, duration: 0.3 }}>
                <h3 className="text-lg">{p.name}</h3>
                <span className="mt-2 inline-block rounded-pill bg-yellow/40 px-3 py-1 text-xs font-semibold">
                  Código: {p.inviteCode}
                </span>
                <p className="mt-2 text-sm text-ink-muted">{p.memberCount} membro(s)</p>
                <div className="mt-4 flex flex-wrap gap-2">
                  <Button size="sm" onClick={() => navigate(`/palpites-do-bolao?poolId=${p.id}`)}>
                    Palpites
                  </Button>
                  <Button size="sm" variant="secondary" onClick={() => navigate("/leaderboard")}>
                    Ranking
                  </Button>
                  {(p.createdBy === user?.id || isAdmin) && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => onDelete(p.id, p.name)}
                      disabled={deletePool.isPending}
                      className="text-danger"
                    >
                      <Trash2 className="h-4 w-4" /> Apagar
                    </Button>
                  )}
                </div>
              </MotionCard>
            ))}
          </div>
        )}
      </div>
    </PageShell>
  );
}

function CloseButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label="Fechar"
      className="-mr-1 -mt-1 inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-ink-muted transition-colors hover:bg-ink/5 hover:text-ink focus-visible:outline-none focus-visible:shadow-glow"
    >
      <X className="h-5 w-5" />
    </button>
  );
}
