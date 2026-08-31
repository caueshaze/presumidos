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
        backgroundColor: option.mediaSeen ? "rgba(68, 201, 161, 0.15)" : "rgba(255, 255, 255, 0.04)",
      }}
      transition={{ duration: 0.2, ease: "easeOut" }}
      className={`ml-auto inline-flex items-center justify-center rounded-pill border px-2.5 py-1.5 font-semibold transition-colors focus-visible:outline-none focus-visible:shadow-glow disabled:cursor-wait ${option.mediaSeen ? "border-mint-dark/50 text-mint-dark" : "border-mint/20 text-ink-muted hover:border-mint-dark/50 hover:text-mint-dark"}`}
    >
      <span className="relative block h-4 w-[7.5rem]">
        <AnimatePresence initial={false}>
          {option.mediaSeen ? (
            <motion.span key="seen" initial={{ opacity: 0, y: 3 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -3 }} transition={{ duration: 0.16, ease: "easeOut" }} className="absolute inset-0 flex items-center justify-center gap-1.5">
              <Check className="h-3.5 w-3.5" aria-hidden="true" />Visto
            </motion.span>
          ) : (
            <motion.span key="unseen" initial={{ opacity: 0, y: 3 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -3 }} transition={{ duration: 0.16, ease: "easeOut" }} className="absolute inset-0 flex items-center justify-center gap-1.5">
              <Eye className="h-3.5 w-3.5" aria-hidden="true" />Marcar como visto
            </motion.span>
          )}
        </AnimatePresence>
      </span>
    </motion.button>
  </div>;
}
