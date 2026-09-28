import { useState } from "react";
import { RotateCcw } from "lucide-react";
import { useReopenPool } from "@/hooks/queries";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/ui/field";

type ReopenPoolActionProps = {
  poolId: string;
  poolName: string;
};

export function ReopenPoolAction({ poolId, poolName }: ReopenPoolActionProps) {
  const reopenPool = useReopenPool();
  const [open, setOpen] = useState(false);
  const error = reopenPool.error instanceof Error ? reopenPool.error.message : "";
  const reopen = async () => {
    try {
      await reopenPool.mutateAsync(poolId);
      setOpen(false);
    } catch {}
  };

  return <>
    <Button variant="outline" size="sm" onClick={() => setOpen(true)}><RotateCcw className="h-4 w-4" />Reabrir bolão</Button>
    {open && <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/45 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !reopenPool.isPending) setOpen(false); }}>
      <div className="w-full max-w-lg rounded-[28px] border border-mint/20 bg-card p-6 shadow-2xl shadow-black/25 sm:p-7" role="dialog" aria-modal="true" aria-labelledby="reopen-pool-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="flex items-start gap-4">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-mint/20 text-mint-dark"><RotateCcw className="h-6 w-6" /></div>
          <div><h2 id="reopen-pool-title" className="text-2xl">Reabrir bolão?</h2><p className="mt-1 text-sm text-ink-muted">{poolName}</p></div>
        </div>
        <div className="mt-6 rounded-2xl border border-mint/15 bg-bg/35 px-4 py-4 text-sm"><strong>O bolão deixará de ser histórico.</strong> Os palpites e novas entradas continuarão encerrados.</div>
        {error && <div className="mt-4"><ErrorBanner>{error}</ErrorBanner></div>}
        <div className="mt-6 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button variant="outline" className="justify-center" onClick={() => setOpen(false)} disabled={reopenPool.isPending}>Cancelar</Button>
          <Button variant="outline" className="justify-center" onClick={() => void reopen()} disabled={reopenPool.isPending}>{reopenPool.isPending ? "Processando..." : "Reabrir bolão"}</Button>
        </div>
      </div>
    </div>}
  </>;
}
