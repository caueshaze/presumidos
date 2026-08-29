import { Check, ExternalLink, Eye } from "lucide-react";
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
    <button
      type="button"
      aria-pressed={option.mediaSeen ?? false}
      aria-label={option.mediaSeen ? "Mídia marcada como vista" : "Marcar mídia como vista"}
      disabled={progress.isPending}
      onClick={(event) => {
        blockSelection(event);
        progress.mutate({ poolId, optionId: option.id, seen: !(option.mediaSeen ?? false) });
      }}
      className={`ml-auto inline-flex items-center gap-1.5 rounded-pill border px-2.5 py-1.5 font-semibold transition-all focus-visible:outline-none focus-visible:shadow-glow disabled:cursor-wait ${option.mediaSeen ? "border-mint-dark/50 bg-mint/15 text-mint-dark" : "border-mint/20 bg-card/45 text-ink-muted hover:border-mint-dark/50 hover:bg-mint/10 hover:text-mint-dark"}`}
    >
      {option.mediaSeen ? <Check className="h-3.5 w-3.5" aria-hidden="true" /> : <Eye className="h-3.5 w-3.5" aria-hidden="true" />}
      {option.mediaSeen ? "Visto" : "Marcar como visto"}
    </button>
  </div>;
}
