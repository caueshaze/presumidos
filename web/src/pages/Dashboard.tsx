import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { AnimatePresence, motion } from "framer-motion";
import { Plus, X, Trash2 } from "lucide-react";
import { usePools, useCreatePool, useJoinPool, useDeletePool } from "@/hooks/queries";
import { useAuth } from "@/hooks/useAuth";
import { PageShell } from "@/components/PageShell";
import { Card, MotionCard } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";

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

  const [showForms, setShowForms] = useState(false);
  const [newPoolName, setNewPoolName] = useState("");
  const [joinCode, setJoinCode] = useState("");
  const [error, setError] = useState("");

  const onCreate = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      await createPool.mutateAsync(newPoolName);
      setNewPoolName("");
      setShowForms(false);
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
      setShowForms(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao entrar no bolão.");
    }
  };

  return (
    <PageShell>
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-3xl">Seus bolões</h1>
        <Button
          variant={showForms ? "outline" : "primary"}
          size="sm"
          onClick={() => {
            setShowForms((v) => !v);
            setError("");
          }}
        >
          {showForms ? (
            <>
              <X className="h-4 w-4" /> Fechar
            </>
          ) : (
            <>
              <Plus className="h-4 w-4" /> Novo bolão
            </>
          )}
        </Button>
      </div>

      {/* Formulários de criar/entrar — recolhidos por padrão */}
      <AnimatePresence initial={false}>
        {showForms && (
          <motion.div
            key="pool-forms"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
            className="overflow-hidden"
          >
            <div className="grid gap-5 pt-5 sm:grid-cols-2">
              <Card>
                <h2 className="text-xl">Criar bolão</h2>
                <form onSubmit={onCreate} className="mt-3 flex flex-col gap-3">
                  <Input
                    placeholder="Nome do bolão"
                    value={newPoolName}
                    onChange={(e) => setNewPoolName(e.target.value)}
                    required
                  />
                  <Button type="submit" disabled={createPool.isPending}>
                    Criar
                  </Button>
                </form>
              </Card>
              <Card>
                <h2 className="text-xl">Entrar com código</h2>
                <form onSubmit={onJoin} className="mt-3 flex flex-col gap-3">
                  <Input
                    placeholder="Código de convite"
                    value={joinCode}
                    onChange={(e) => setJoinCode(e.target.value)}
                    required
                  />
                  <Button type="submit" variant="secondary" disabled={joinPool.isPending}>
                    Entrar
                  </Button>
                </form>
              </Card>
            </div>
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
              Crie um bolão para reunir a galera ou entre com um código para começar a palpitar.
            </p>
            <Button className="mt-4" onClick={() => setShowForms(true)}>
              <Plus className="h-4 w-4" /> Criar ou entrar em um bolão
            </Button>
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
                  <Button size="sm" onClick={() => navigate("/predictions")}>
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
