import { Check, Copy, KeyRound, Link2, Share2, Sparkles, X } from "lucide-react";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";

export type ShareCopyTarget = "link" | "code";

type PoolShareModalProps = {
  inviteUrl: string;
  inviteCode: string;
  poolName: string;
  copied: ShareCopyTarget | null;
  canShare: boolean;
  onCopy: (value: string, target: ShareCopyTarget) => void;
  onShare: () => void;
  onClose: () => void;
};

export function PoolShareModal({ inviteUrl, inviteCode, poolName, copied, canShare, onCopy, onShare, onClose }: PoolShareModalProps) {
  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.2, ease: "easeOut" }} className="fixed inset-0 z-50 flex items-center justify-center bg-ink/45 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <motion.div initial={{ opacity: 0, y: 18, scale: 0.97 }} animate={{ opacity: 1, y: 0, scale: 1 }} transition={{ duration: 0.3, ease: [0.22, 1, 0.36, 1] }} className="relative w-full max-w-lg overflow-hidden rounded-[28px] border border-mint/20 bg-card shadow-2xl shadow-black/25" role="dialog" aria-modal="true" aria-labelledby="share-pool-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="pointer-events-none absolute inset-x-0 top-0 h-36 bg-gradient-to-br from-mint/25 via-mint/5 to-transparent" />
        <div className="relative p-5 sm:p-7">
          <div className="flex items-start gap-4">
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-mint/20 text-mint-dark shadow-inner shadow-mint/10"><Share2 className="h-6 w-6" /></div>
            <div className="min-w-0 flex-1"><h2 id="share-pool-title" className="text-2xl leading-tight">Compartilhar bolão</h2><p className="mt-1 text-sm text-ink-muted">Convide seus amigos para participar de “{poolName}”.</p></div>
            <Button variant="link" size="sm" className="h-10 w-10 shrink-0 rounded-full p-0 text-ink-muted hover:bg-mint/10 hover:no-underline" aria-label="Fechar compartilhamento" onClick={onClose}><X className="h-5 w-5" /></Button>
          </div>
          <div className="mt-6 grid gap-3">
            <div className="rounded-2xl border border-mint/15 bg-bg/35 p-4 transition-colors hover:border-mint/35 hover:bg-bg/55"><div className="flex items-start justify-between gap-3"><div className="flex min-w-0 items-center gap-3"><div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-sky/15 text-sky-dark"><Link2 className="h-5 w-5" /></div><div className="min-w-0"><p className="font-semibold">Link de convite</p></div></div><Button variant="outline" size="sm" className="shrink-0" aria-label={copied === "link" ? "Link copiado" : "Copiar link"} onClick={() => onCopy(inviteUrl, "link")}>{copied === "link" ? <Check className="h-4 w-4 text-mint-dark" /> : <Copy className="h-4 w-4" />}{copied === "link" ? "Copiado" : "Copiar link"}</Button></div></div>
            <div className="rounded-2xl border border-mint/15 bg-bg/35 p-4 transition-colors hover:border-mint/35 hover:bg-bg/55"><div className="flex items-start justify-between gap-3"><div className="flex items-center gap-3"><div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-yellow/20 text-yellow-dark"><KeyRound className="h-5 w-5" /></div><div><p className="font-semibold">Código do bolão</p></div></div><Button variant="outline" size="sm" className="shrink-0" aria-label={copied === "code" ? "Código copiado" : "Copiar código"} onClick={() => onCopy(inviteCode, "code")}>{copied === "code" ? <Check className="h-4 w-4 text-mint-dark" /> : <Copy className="h-4 w-4" />}{copied === "code" ? "Copiado" : "Copiar código"}</Button></div><code className="mt-3 inline-block rounded-xl border border-yellow/20 bg-yellow/15 px-3 py-2 font-heading text-lg font-semibold tracking-[0.2em] text-ink">{inviteCode}</code></div>
          </div>
          <div className="mt-5 flex items-start gap-3 rounded-2xl bg-mint/10 px-4 py-3 text-sm text-ink-muted"><Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-mint-dark" /><p>Compartilhe o link para facilitar a entrada. O código também funciona na opção “Entrar com código”.</p></div>
          <div className="mt-5 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end"><Button variant="outline" className="justify-center" onClick={onClose}>Fechar</Button>{canShare && <Button className="justify-center" onClick={onShare}><Share2 className="h-4 w-4" />Compartilhar pelo dispositivo</Button>}</div>
        </div>
      </motion.div>
    </motion.div>
  );
}
