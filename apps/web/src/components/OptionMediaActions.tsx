import { Check, ExternalLink, Eye } from "lucide-react";
import { AnimatePresence, motion } from "framer-motion";
import type { SyntheticEvent } from "react";
import { useUpdateOptionMediaProgress } from "@/hooks/queries";
import type { CustomQuestionOption } from "@/types";

export function OptionMediaActions({ option, poolId }: { option: CustomQuestionOption; poolId: string }) {
  const progress = useUpdateOptionMediaProgress();
  if (!option.links?.length) return null;
  const blockSelection = (event: SyntheticEvent) => event.stopPropagation();
  return <div className="mt-3 flex flex-wrap items-center justify-between gap-2 text-xs" onClick={blockSelection}>
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
      {option.links.map((link) => <a key={`${link.kind}-${link.url}`} href={link.url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 font-semibold text-mint-dark underline-offset-2 hover:underline" onClick={blockSelection}><ExternalLink className="h-3.5 w-3.5" />{link.label}</a>)}
    </div>
    <motion.button
      type="button"
      aria-pressed={option.mediaSeen ?? false}
      aria-label={option.mediaSeen ? "Mídia marcada como vista" : "Marcar mídia como vista"}
      disabled={progress.isPending}
      onClick={(event) => {
        blockSelection(event);
        progress.mutate({ poolId, optionId: option.id, seen: !(option.mediaSeen ?? false) });
      }}
      animate={{
        scale: option.mediaSeen ? 1.025 : 1,
        backgroundColor: option.mediaSeen ? "rgba(68, 201, 161, 0.15)" : "rgba(255, 255, 255, 0.04)",
      }}
      whileHover={progress.isPending ? undefined : { scale: option.mediaSeen ? 1.035 : 1.02 }}
      whileTap={progress.isPending ? undefined : { scale: 0.97 }}
      transition={{ type: "spring", stiffness: 380, damping: 24, mass: 0.55 }}
      className={`ml-auto inline-flex items-center gap-1.5 rounded-pill border px-2.5 py-1.5 font-semibold focus-visible:outline-none focus-visible:shadow-glow disabled:cursor-wait ${option.mediaSeen ? "border-mint-dark/50 text-mint-dark" : "border-mint/20 text-ink-muted hover:border-mint-dark/50 hover:text-mint-dark"}`}
    >
      <AnimatePresence initial={false} mode="wait">
        {option.mediaSeen ? (
          <motion.span key="seen" initial={{ opacity: 0, scale: 0.5, rotate: -20 }} animate={{ opacity: 1, scale: 1, rotate: 0 }} exit={{ opacity: 0, scale: 0.5, rotate: 20 }} transition={{ type: "spring", stiffness: 500, damping: 22 }} className="flex items-center gap-1.5">
            <Check className="h-3.5 w-3.5" aria-hidden="true" />Visto
          </motion.span>
        ) : (
          <motion.span key="unseen" initial={{ opacity: 0, scale: 0.85 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 0.85 }} transition={{ duration: 0.14 }} className="flex items-center gap-1.5">
            <Eye className="h-3.5 w-3.5" aria-hidden="true" />Marcar como visto
          </motion.span>
        )}
      </AnimatePresence>
    </motion.button>
  </div>;
}
