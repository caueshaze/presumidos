import type { FormEvent } from "react";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";
import { PtBrDateTimeInput } from "@/components/PtBrDateTimeInput";

type CreateProps = {
  navigate: (path: string) => void; name: string; setName: (value: string) => void;
  startsAt: string; setStartsAt: (value: string) => void; endsAt: string; setEndsAt: (value: string) => void;
  create: (event: FormEvent) => Promise<void>; busy: boolean; error: string;
};

export function CreateEvent({ navigate, name, setName, startsAt, setStartsAt, endsAt, setEndsAt, create, busy, error }: CreateProps) {
  return <PageShell><Button variant="link" size="sm" onClick={() => navigate("/events")}>← Voltar aos eventos</Button><h1 className="mt-3 text-3xl">Criar evento</h1><Card className="mt-5"><form onSubmit={create} className="flex flex-col gap-3"><label>Nome do evento<Input value={name} onChange={(event) => setName(event.target.value)} required /></label><label>Data inicial<PtBrDateTimeInput value={startsAt} onChange={setStartsAt} /></label><label>Data final<PtBrDateTimeInput value={endsAt} onChange={setEndsAt} /></label><Button disabled={busy}>Criar rascunho</Button></form>{error && <p role="alert" aria-live="polite" className="mt-3 rounded-lg bg-danger-bg/50 px-3 py-2 text-sm font-medium text-danger">{error}</p>}</Card></PageShell>;
}

export function EventLoading({ navigate, error }: Pick<CreateProps, "navigate" | "error">) {
  return <PageShell><Button variant="link" size="sm" onClick={() => navigate("/events")}>← Voltar aos eventos</Button><p className="mt-3">Carregando evento…</p>{error && <ErrorBanner>{error}</ErrorBanner>}</PageShell>;
}
