import { AlertCircle, Inbox, LoaderCircle } from "lucide-react";
import { Button } from "./button";
import { Card } from "./card";

export function LoadingState({ label = "Carregando..." }: { label?: string }) {
  return <Card className="flex items-center gap-3 text-ink-muted"><LoaderCircle className="h-5 w-5 animate-spin" aria-hidden="true" />{label}</Card>;
}

export function EmptyState({ title, children, action }: { title: string; children: React.ReactNode; action?: React.ReactNode }) {
  return <Card className="text-center"><Inbox className="mx-auto h-7 w-7 text-mint-dark" aria-hidden="true" /><h2 className="mt-3 text-xl">{title}</h2><p className="mx-auto mt-2 max-w-xl text-ink-muted">{children}</p>{action && <div className="mt-4">{action}</div>}</Card>;
}

export function ErrorState({ onRetry, children = "Não foi possível carregar esta página." }: { onRetry?: () => void; children?: React.ReactNode }) {
  return <Card className="border-danger/30"><div className="flex gap-3"><AlertCircle className="mt-0.5 h-5 w-5 shrink-0 text-danger" aria-hidden="true" /><div><h2 className="text-lg">Algo deu errado</h2><p className="mt-1 text-ink-muted">{children}</p>{onRetry && <Button className="mt-3" variant="outline" size="sm" onClick={onRetry}>Tentar novamente</Button>}</div></div></Card>;
}

export function ProgressBar({ value, total }: { value: number; total: number }) {
  const percent = total > 0 ? Math.round((value / total) * 100) : 0;
  return <div className="mt-2" aria-label={`${value} de ${total} palpites respondidos`}><div className="h-2 overflow-hidden rounded-full bg-mint/20"><div className="h-full rounded-full bg-mint-dark transition-[width]" style={{ width: `${percent}%` }} /></div><p className="mt-1 text-sm text-ink-muted">{value} de {total} palpites respondidos</p></div>;
}
