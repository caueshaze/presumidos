import { useEffect, useMemo, useState } from "react";
import { Navigate } from "react-router-dom";
import { motion } from "framer-motion";
import { UserPlus, X } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import {
  useAdminPools,
  useAdminUsers,
  useAdminPoolMembers,
  useAddPoolMember,
  useRemovePoolMember,
  useReauth,
} from "@/hooks/queries";
import { withAdminReauth } from "@/lib/adminReauth";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label, Select, ErrorBanner } from "@/components/ui/field";

export function AdminPage() {
  const { isAdmin, loading } = useAuth();

  const pools = useAdminPools();
  const users = useAdminUsers();
  const [selectedPool, setSelectedPool] = useState("");
  const [selectedUser, setSelectedUser] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    if (!selectedPool && pools.data && pools.data.length > 0) {
      setSelectedPool(pools.data[0].id);
    }
  }, [pools.data, selectedPool]);

  const members = useAdminPoolMembers(selectedPool || null);
  const addMember = useAddPoolMember();
  const removeMember = useRemovePoolMember();
  const reauth = useReauth();

  // Usuários que ainda não estão no bolão selecionado.
  const availableUsers = useMemo(() => {
    const memberIds = new Set((members.data ?? []).map((m) => m.id));
    return (users.data ?? []).filter((u) => !memberIds.has(u.id));
  }, [users.data, members.data]);

  useEffect(() => {
    // Reseta a seleção de usuário quando a lista de disponíveis muda.
    setSelectedUser("");
  }, [selectedPool, members.data]);

  if (!loading && !isAdmin) return <Navigate to="/" replace />;

  const onAdd = async () => {
    if (!selectedPool || !selectedUser) return;
    setError("");
    try {
      await withAdminReauth(
        () => addMember.mutateAsync({ poolId: selectedPool, userId: selectedUser }),
        (password) => reauth.mutateAsync(password),
      );
      setSelectedUser("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao adicionar usuário.");
    }
  };

  const onRemove = async (userId: string) => {
    if (!selectedPool) return;
    setError("");
    try {
      await withAdminReauth(
        () => removeMember.mutateAsync({ poolId: selectedPool, userId }),
        (password) => reauth.mutateAsync(password),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao remover usuário.");
    }
  };

  return (
    <PageShell>
      <h1 className="text-3xl">Administração</h1>
      <p className="mt-2 max-w-3xl text-sm text-ink-muted">
        Gerencie os membros dos bolões já criados: adicione ou remova participantes de qualquer
        bolão existente.
      </p>

      {pools.isLoading ? (
        <Card className="mt-6">
          <p className="text-ink-muted">Carregando...</p>
        </Card>
      ) : pools.isError ? (
        <div className="mt-6">
          <ErrorBanner>Erro ao carregar bolões: {(pools.error as Error).message}</ErrorBanner>
        </div>
      ) : pools.data && pools.data.length === 0 ? (
        <Card className="mt-6">
          <h3 className="text-lg">Nenhum bolão criado ainda.</h3>
          <p className="mt-1 text-ink-muted">
            Crie um bolão primeiro para depois gerenciar seus membros aqui.
          </p>
        </Card>
      ) : (
        <>
          <Card className="mt-6 max-w-sm">
            <Label htmlFor="pool-select">Bolão</Label>
            <Select
              id="pool-select"
              value={selectedPool}
              onChange={(e) => setSelectedPool(e.target.value)}
            >
              {pools.data?.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name} ({p.memberCount})
                </option>
              ))}
            </Select>
          </Card>

          {error && (
            <div className="mt-4">
              <ErrorBanner>{error}</ErrorBanner>
            </div>
          )}

          {/* Adicionar membro */}
          <Card className="mt-5">
            <h2 className="text-xl">Adicionar usuário ao bolão</h2>
            {users.isLoading ? (
              <p className="mt-2 text-ink-muted">Carregando usuários...</p>
            ) : availableUsers.length === 0 ? (
              <p className="mt-2 text-sm text-ink-muted">
                Todos os usuários já estão neste bolão.
              </p>
            ) : (
              <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-end">
                <div className="flex-1">
                  <Label htmlFor="user-select">Usuário</Label>
                  <Select
                    id="user-select"
                    value={selectedUser}
                    onChange={(e) => setSelectedUser(e.target.value)}
                  >
                    <option value="">Selecione um usuário</option>
                    {availableUsers.map((u) => (
                      <option key={u.id} value={u.id}>
                        {u.username} — {u.email}
                      </option>
                    ))}
                  </Select>
                </div>
                <Button
                  onClick={onAdd}
                  disabled={!selectedUser || addMember.isPending}
                  className="self-start sm:self-auto"
                >
                  <UserPlus className="h-4 w-4" />
                  {addMember.isPending ? "Adicionando..." : "Adicionar"}
                </Button>
              </div>
            )}
          </Card>

          {/* Membros atuais */}
          <Card className="mt-5">
            <h2 className="text-xl">Membros atuais</h2>
            {members.isLoading ? (
              <p className="mt-2 text-ink-muted">Carregando...</p>
            ) : members.isError ? (
              <div className="mt-3">
                <ErrorBanner>
                  Erro ao carregar membros: {(members.error as Error).message}
                </ErrorBanner>
              </div>
            ) : (members.data?.length ?? 0) === 0 ? (
              <p className="mt-2 text-sm text-ink-muted">Este bolão ainda não tem membros.</p>
            ) : (
              <ul className="mt-3 divide-y divide-mint/20">
                {members.data?.map((m, i) => (
                  <motion.li
                    key={m.id}
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: i * 0.04, duration: 0.25 }}
                    className="flex items-center justify-between gap-3 py-3"
                  >
                    <div className="min-w-0">
                      <div className="truncate font-heading font-semibold text-ink">
                        {m.username}
                        {m.isAdmin && (
                          <span className="ml-2 rounded-pill bg-yellow/40 px-2 py-0.5 text-xs font-semibold">
                            admin
                          </span>
                        )}
                      </div>
                      <div className="truncate text-xs text-ink-muted">{m.email}</div>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => onRemove(m.id)}
                      disabled={removeMember.isPending}
                      className="shrink-0"
                    >
                      <X className="h-4 w-4" /> Remover
                    </Button>
                  </motion.li>
                ))}
              </ul>
            )}
          </Card>
        </>
      )}
    </PageShell>
  );
}
