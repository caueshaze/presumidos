import { ExternalLink } from "lucide-react";
import type { SyntheticEvent } from "react";
import { useUpdateOptionMediaProgress } from "@/hooks/queries";
import type { CustomQuestionOption } from "@/types";

export function OptionMediaActions({ option, poolId }: { option: CustomQuestionOption; poolId: string }) {
  const progress = useUpdateOptionMediaProgress();
  if (!option.links?.length) return null;
  const blockSelection = (event: SyntheticEvent) => event.stopPropagation();
  return <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs" onClick={blockSelection}>
    {option.links.map((link) => <a key={`${link.kind}-${link.url}`} href={link.url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 font-semibold text-mint-dark underline-offset-2 hover:underline" onClick={blockSelection}><ExternalLink className="h-3.5 w-3.5" />{link.label}</a>)}
    <label className="inline-flex cursor-pointer items-center gap-1.5 text-ink-muted" onClick={blockSelection}>
      <input type="checkbox" checked={option.mediaSeen ?? false} disabled={progress.isPending} onChange={(event) => progress.mutate({ poolId, optionId: option.id, seen: event.target.checked })} className="h-3.5 w-3.5 accent-[var(--color-mint-dark)]" />
      {option.mediaSeen ? "Visto" : "Marcar como visto"}
    </label>
  </div>;
}
