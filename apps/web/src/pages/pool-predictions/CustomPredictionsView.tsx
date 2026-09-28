// @ts-nocheck
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label, Select } from "@/components/ui/field";
import { initials } from "./PredictionDetail";

export function CustomPredictionsView({ context }: { context: Record<string, any> }) {
  const { navigate, selectedPool, currentPool, setSelectedPool, pools, customMembers,
    selectedMemberId, showPoolSelector } = context;
  const members = customMembers.data ?? [];
  const selectedMember = members.find((member) => member.userId === selectedMemberId);
  const membersUrl = `/pools/${encodeURIComponent(selectedPool)}/members`;

  return <PageShell>
    <Button variant="link" size="sm" onClick={() => navigate(selectedMember ? membersUrl : `/pools/${selectedPool}`)}>← Voltar ao bolão</Button>
    <h1 className="mt-3 text-3xl">{selectedMember ? `Palpites de ${selectedMember.username}` : showPoolSelector ? "Participantes do bolão" : "Participantes"}</h1>
    {currentPool && <p className="mt-1 text-ink-muted">Bolão: {currentPool.name} · Evento: {currentPool.event.name}</p>}
    {showPoolSelector && !selectedMember && <Card className="mt-6 max-w-sm"><Label htmlFor="custom-pool-select">Bolão</Label><Select id="custom-pool-select" value={selectedPool} onChange={(event) => setSelectedPool(event.target.value)}>{pools.data?.map((pool) => <option key={pool.id} value={pool.id}>{pool.name} · {pool.event.name}</option>)}</Select></Card>}
    <div className="mt-5">
      {customMembers.isLoading ? <Card><p className="text-ink-muted">Carregando...</p></Card> : selectedMember ? <MemberDetail member={selectedMember} /> : members.length === 0 ? <Card><p className="text-ink-muted">Nenhum participante para mostrar.</p></Card> : <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">{members.map((member) => <button key={member.userId} type="button" onClick={() => navigate(`${membersUrl}/${encodeURIComponent(member.userId)}`)} className="flex items-center gap-3 rounded-lg bg-card p-4 text-left shadow-card transition-shadow hover:shadow-card-hover"><span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-mint/40 font-heading font-bold text-mint-dark">{initials(member.username)}</span><div><p className="font-heading font-semibold">{member.username}</p><p className="text-xs text-ink-muted">{member.predictions.length} {member.predictions.length === 1 ? "palpite visível" : "palpites visíveis"}</p></div></button>)}</div>}
    </div>
  </PageShell>;
}

function MemberDetail({ member }: { member: any }) {
  return <Card>
    <div className="flex items-center gap-3">
      <span className="flex h-12 w-12 items-center justify-center rounded-full bg-mint/40 font-heading text-lg font-bold text-mint-dark">{initials(member.username)}</span>
      <div><h2 className="font-heading text-xl">{member.username}</h2><p className="text-sm text-ink-muted">{member.predictions.length} {member.predictions.length === 1 ? "palpite visível" : "palpites visíveis"}</p></div>
    </div>
    {member.predictions.length === 0 ? <p className="mt-4 text-sm text-ink-muted">Palpites ainda ocultos.</p> : <div className="mt-4 space-y-3">{member.predictions.map((prediction) => <div key={prediction.itemId} className="flex items-start justify-between gap-3 border-t border-mint/15 pt-3 first:border-0 first:pt-0"><div><p className="text-sm text-ink-muted">{prediction.title}</p><p className="font-semibold">{prediction.optionLabel}</p></div>{typeof prediction.points === "number" && <span className={prediction.points > 0 ? "shrink-0 rounded-pill bg-success/15 px-2.5 py-0.5 text-xs font-semibold text-mint-dark ring-1 ring-success/35" : "shrink-0 rounded-pill bg-card px-2.5 py-0.5 text-xs font-semibold text-ink-muted ring-1 ring-mint/25"}>{prediction.points > 0 ? `+${prediction.points} pts` : "0 pts"}</span>}</div>)}</div>}
  </Card>;
}
