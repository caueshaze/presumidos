import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { useCustomQuestions, useFootballScoring, useMatches, useMyEvents, usePools, useSetCustomResult, useUpdateCustomScoring, useUpdateFootballScoring } from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner, Label } from "@/components/ui/field";

export function PoolScoringPage() {
  const { poolId = "" } = useParams();
  const navigate = useNavigate(); const { user, isAdmin } = useAuth(); const pools = usePools();
  const matches = useMatches();
  const events = useMyEvents();
  const pool = pools.data?.find((item) => item.id === poolId);
  const custom = useCustomQuestions(pool?.event.kind === "custom" ? poolId : null);
  const football = useFootballScoring(pool?.event.kind === "football" ? poolId : null);
  const updateFootball = useUpdateFootballScoring(); const updateCustom = useUpdateCustomScoring();
  const setResult = useSetCustomResult();
  const [values, setValues] = useState<Record<string, string>>({}); const [error, setError] = useState("");
  useEffect(() => { if (football.data) setValues(Object.fromEntries(Object.entries(football.data).map(([k,v]) => [k, String(v)]))); }, [football.data]);
  const owner = pool?.createdBy === user?.id || isAdmin;
  const eventOwner = isAdmin || events.data?.some((event) => event.id === pool?.eventId) === true;
  if (pools.isLoading) return <PageShell><Card><p className="text-ink-muted">Carregando...</p></Card></PageShell>;
  if (!pool) return <PageShell><ErrorBanner>Bolão não encontrado.</ErrorBanner></PageShell>;
  const footballFrozen = pool.event.kind === "football" && (matches.data ?? []).some((match) => new Date(match.kickoff).getTime() <= Date.now());
  const saveFootball = async () => { setError(""); try { await updateFootball.mutateAsync({ poolId, exactScorePoints:+values.exactScorePoints, correctResultExactSidePoints:+values.correctResultExactSidePoints, correctResultPoints:+values.correctResultPoints, incorrectResultPoints:+values.incorrectResultPoints, knockoutBonusPoints:+values.knockoutBonusPoints }); } catch (e) { setError(e instanceof Error ? e.message : "Falha ao salvar."); } };
  return <PageShell><Button variant="link" size="sm" onClick={() => navigate(`/pools/${poolId}/predictions`)}>← Voltar aos palpites</Button><h1 className="mt-3 text-3xl">Regras de pontuação</h1><p className="mt-1 text-ink-muted">{pool.name} · {pool.event.name}</p>
    {pool.event.kind === "football" ? <Card className="mt-6 max-w-xl">{footballFrozen && owner && <p className="mb-4 text-sm text-ink-muted">A pontuação não pode mais ser alterada porque os primeiros palpites já foram encerrados.</p>}<div className="space-y-3">{[["exactScorePoints","Placar exato"],["correctResultExactSidePoints","Resultado + lado exato"],["correctResultPoints","Resultado correto"],["incorrectResultPoints","Erro"],["knockoutBonusPoints","Bônus mata-mata"]].map(([key,label]) => <div key={key} className="flex items-center justify-between gap-4"><Label>{label}</Label>{owner && !footballFrozen ? <Input className="w-24" type="number" min="0" value={values[key] ?? ""} onChange={(e) => setValues({...values,[key]:e.target.value})} /> : <strong>{values[key] ?? "—"} pts</strong>}</div>)}</div>{owner && !footballFrozen && <Button className="mt-5" onClick={saveFootball} disabled={updateFootball.isPending}>Salvar pontuação</Button>}</Card> : <><Card className="mt-6"><p className="text-sm text-ink-muted">Cada categoria usa a pontuação deste bolão.</p><div className="mt-4 space-y-3">{custom.data?.map((q) => <CustomScoringRow key={q.itemId} question={q} owner={owner && q.status === "open"} save={(correctPoints, incorrectPoints) => updateCustom.mutateAsync({poolId,itemId:q.itemId,correctPoints,incorrectPoints})}/>)}</div></Card>{eventOwner && <Card className="mt-5"><h2 className="text-xl">Resultados oficiais do evento</h2><p className="mt-1 text-sm text-ink-muted">Esta ação é global para o Event. O dono do bolão não altera vencedores sem também ser dono do Event.</p><div className="mt-4 space-y-3">{custom.data?.map((q) => <CustomResultRow key={q.itemId} question={q} save={(optionId) => setResult.mutateAsync({poolId,itemId:q.itemId,optionId})}/>)}</div></Card>}</>}{error && <div className="mt-4"><ErrorBanner>{error}</ErrorBanner></div>}</PageShell>;
}
function CustomScoringRow({question, owner, save}:{question: import("@/types").CustomQuestion; owner:boolean; save:(correct:number,incorrect:number)=>Promise<unknown>}) { const [correct,setCorrect]=useState(String(question.correctPoints)); const [incorrect,setIncorrect]=useState(String(question.incorrectPoints)); return <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3 last:border-0"><span className="font-medium">{question.title}</span>{owner ? <div className="flex items-center gap-2"><Input aria-label={`Pontos por acerto: ${question.title}`} className="w-16" type="number" min="0" value={correct} onChange={e=>setCorrect(e.target.value)}/><Input aria-label={`Pontos por erro: ${question.title}`} className="w-16" type="number" min="0" value={incorrect} onChange={e=>setIncorrect(e.target.value)}/><Button size="sm" variant="outline" onClick={()=>save(+correct,+incorrect)}>Salvar</Button></div> : <span className="text-sm text-ink-muted">{question.status !== "open" ? "Palpites encerrados; pontuação somente leitura." : `${question.correctPoints} pts por acerto`}</span>}</div> }
function CustomResultRow({question, save}:{question: import("@/types").CustomQuestion; save:(optionId:string)=>Promise<unknown>}) { const [optionId,setOptionId]=useState(question.correctOptionId ?? ""); return <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3 last:border-0"><Label>{question.title}</Label><div className="flex gap-2"><select aria-label={`Vencedor: ${question.title}`} className="rounded-lg border border-mint/25 bg-card px-2 py-1" value={optionId} onChange={e=>setOptionId(e.target.value)}><option value="">Selecione</option>{question.options.map(option=><option key={option.id} value={option.id}>{option.label}</option>)}</select><Button size="sm" disabled={!optionId} onClick={()=>save(optionId)}>Salvar resultado</Button></div></div> }
