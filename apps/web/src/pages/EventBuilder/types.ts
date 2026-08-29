import type { EventVersionHistory } from "@/types";
import type { Item } from "@/components/EventBuilderItems";

export type Draft = {
  event: {
    id: string;
    name: string;
    status: "draft" | "active";
    createdBy: string | null;
    startsAt: string | null;
    endsAt: string | null;
    description: string | null;
    coverUrl: string | null;
    coverAssetId?: string | null;
    coverAssetUrl?: string | null;
    externalUrl: string | null;
  };
  items: Item[];
  versions: EventVersionHistory[];
};

export function eventCreationError(cause: unknown): string {
  const message = cause instanceof Error ? cause.message : "";
  if (/startsAt.*deve preceder.*endsAt/i.test(message)) return "A data inicial deve ser anterior à data final.";
  if (/startsAt.*inválido/i.test(message)) return "Confira a data inicial do evento.";
  if (/endsAt.*inválido/i.test(message)) return "Confira a data final do evento.";
  return message && /Nome do evento/i.test(message)
    ? message
    : "Não foi possível criar o rascunho. Confira os dados e tente novamente.";
}
