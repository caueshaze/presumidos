import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/ui/field";

export type PredictionReuseModalSuggestion = { sourcePool: { name: string } | null; answered: number; copyable: number; total: number; locked: number };

type PredictionReuseModalProps = {
  suggestion: PredictionReuseModalSuggestion;
  pending: boolean;
  error: string;
  onCopy: () => void;
  onStartEmpty: () => void;
  onClose: () => void;
};

export function PredictionReuseModal({ suggestion, pending, error, onCopy, onStartEmpty, onClose }: PredictionReuseModalProps) {
  const source = suggestion.sourcePool?.name ?? "outro bolão";
  return <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/45 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !pending) onClose(); }}>
    <div className="w-full max-w-lg rounded-[28px] border border-mint/20 bg-card p-6 shadow-2xl shadow-black/25 sm:p-7" role="dialog" aria-modal="true" aria-labelledby="prediction-reuse-title" onMouseDown={(event) => event.stopPropagation()}>
      <div className="flex items-start justify-between gap-4"><div><h2 id="prediction-reuse-title" className="text-2xl">Você já tem palpites para este evento</h2><p className="mt-2 text-sm text-ink-muted">Encontramos {suggestion.answered} de {suggestion.total} palpites feitos no “{source}”.</p></div><Button variant="link" size="sm" className="h-10 w-10 shrink-0 rounded-full p-0" aria-label="Fechar" disabled={pending} onClick={onClose}><X className="h-5 w-5" /></Button></div>
      {suggestion.locked > 0 && <p className="mt-4 rounded-2xl bg-yellow/15 px-4 py-3 text-sm text-ink-muted">{suggestion.copyable} ainda podem ser reutilizados neste bolão. {suggestion.locked} já estão bloqueados.</p>}
      <div className="mt-6 space-y-3"><Button className="h-auto w-full justify-start whitespace-normal px-5 py-4 text-left" disabled={pending} onClick={onCopy}><span><span className="block text-base">{pending ? "Copiando palpites…" : `Usar ${suggestion.copyable} palpites já feitos`}</span><span className="mt-1 block text-sm font-normal opacity-90">Eles serão apenas copiados. Depois você poderá alterá-los neste bolão sem afetar os outros.</span></span></Button><Button variant="outline" className="w-full" disabled={pending} onClick={onStartEmpty}>Começar do zero</Button></div>
      {error && <div className="mt-4"><ErrorBanner>{error}</ErrorBanner></div>}
    </div>
  </div>;
}
