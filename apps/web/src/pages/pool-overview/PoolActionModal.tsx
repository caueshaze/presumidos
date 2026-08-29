import { CheckCircle2, Flag, LogOut, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import type { PoolReportCategory } from "@/types";

export type PoolAction = "report" | "reportSubmitted" | "leave" | "delete";

const reportCategoryOptions: Array<{ value: PoolReportCategory; label: string }> = [
  { value: "inappropriate_content", label: "Conteúdo inadequado" },
  { value: "spam_or_fraud", label: "Spam ou fraude" },
  { value: "harassment", label: "Assédio" },
  { value: "other", label: "Outro" },
];

export function PoolActionModal({
  action,
  poolName,
  reportCategory,
  reportDetails,
  reportPending,
  actionPending,
  error,
  onCategoryChange,
  onDetailsChange,
  onReport,
  onLeave,
  onDelete,
  onClose,
}: {
  action: PoolAction;
  poolName: string;
  reportCategory: PoolReportCategory;
  reportDetails: string;
  reportPending: boolean;
  actionPending: boolean;
  error: string;
  onCategoryChange: (value: PoolReportCategory) => void;
  onDetailsChange: (value: string) => void;
  onReport: () => void;
  onLeave: () => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  const submitted = action === "reportSubmitted";
  const report = action === "report" || submitted;
  const destructive = action === "leave" || action === "delete";
  const title = action === "delete" ? "Excluir bolão" : action === "leave" ? "Sair do bolão" : "Denunciar bolão";
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/45 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !actionPending && !reportPending) onClose(); }}>
      <div className="w-full max-w-lg rounded-[28px] border border-mint/20 bg-card p-6 shadow-2xl shadow-black/25 sm:p-7" role="dialog" aria-modal="true" aria-labelledby="pool-action-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="flex items-start gap-4">
          <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl ${destructive ? "bg-danger/15 text-danger" : "bg-mint/20 text-mint-dark"}`}>
            {action === "delete" ? <Trash2 className="h-6 w-6" /> : action === "leave" ? <LogOut className="h-6 w-6" /> : <Flag className="h-6 w-6" />}
          </div>
          <div className="min-w-0 flex-1">
            <h2 id="pool-action-title" className="text-2xl">{title}</h2>
            <p className="mt-1 text-sm text-ink-muted">{poolName}</p>
          </div>
          <Button variant="link" size="sm" className="h-10 w-10 shrink-0 rounded-full p-0 text-ink-muted hover:bg-mint/10 hover:no-underline" aria-label="Fechar" onClick={onClose} disabled={actionPending || reportPending}><X className="h-5 w-5" /></Button>
        </div>

        {submitted ? (
          <div className="mt-6 rounded-2xl border border-success/35 bg-mint/15 p-4 text-sm font-semibold text-mint-dark"><CheckCircle2 className="mr-2 inline h-5 w-5" />Denúncia enviada para análise.</div>
        ) : report ? (
          <form className="mt-6 space-y-4" onSubmit={(event) => { event.preventDefault(); onReport(); }}>
            <div>
              <Label htmlFor="pool-report-category">Motivo</Label>
              <Select id="pool-report-category" value={reportCategory} onChange={(event) => onCategoryChange(event.target.value as PoolReportCategory)}>
                {reportCategoryOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </Select>
            </div>
            <div>
              <Label htmlFor="pool-report-details">Detalhes (opcional)</Label>
              <textarea id="pool-report-details" value={reportDetails} maxLength={1000} onChange={(event) => onDetailsChange(event.target.value)} placeholder="Conte o que aconteceu..." className="min-h-28 w-full resize-y rounded-md border-2 border-mint/40 bg-card px-4 py-3 text-sm text-ink focus:border-mint-dark focus:outline-none focus:shadow-glow" />
              <p className="mt-1 text-right text-xs text-ink-muted">{reportDetails.length}/1000</p>
            </div>
            {error && <ErrorBanner>{error}</ErrorBanner>}
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
              <Button type="button" variant="outline" className="justify-center" onClick={onClose} disabled={reportPending}>Cancelar</Button>
              <Button type="submit" className="justify-center" disabled={reportPending}>{reportPending ? "Enviando..." : "Enviar denúncia"}</Button>
            </div>
          </form>
        ) : (
          <div className="mt-6 space-y-5">
            <div className={`rounded-2xl border px-4 py-4 text-sm ${destructive ? "border-danger/25 bg-danger-bg" : "border-mint/15 bg-bg/35"}`}>
              {action === "leave" ? "Você perderá o acesso ao bolão. Seus palpites e dados serão preservados caso entre novamente pelo código." : <><strong>Esta ação não pode ser desfeita automaticamente.</strong> Todos os participantes perderão o acesso e o bolão será excluído.</>}
            </div>
            {error && <ErrorBanner>{error}</ErrorBanner>}
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
              <Button variant="outline" className="justify-center" onClick={onClose} disabled={actionPending}>Cancelar</Button>
              <Button variant={action === "delete" ? "primary" : "outline"} className={action === "delete" ? "justify-center bg-danger text-white hover:bg-danger/90" : "justify-center"} onClick={action === "delete" ? onDelete : onLeave} disabled={actionPending}>{actionPending ? "Processando..." : action === "delete" ? "Excluir bolão" : "Sair do bolão"}</Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

