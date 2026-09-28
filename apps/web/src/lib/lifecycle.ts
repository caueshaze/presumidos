import type { EventSummary, PoolSummary } from "@/types";

/** A única tradução de lifecycle de domínio para linguagem da interface. */
export type PresentationStatus = "draft" | "published" | "active" | "locked" | "finished";

export function eventPresentationStatus(event: Pick<EventSummary, "status" | "isHistorical">): PresentationStatus {
  if (event.status === "draft") return "draft";
  if (event.isHistorical || event.status === "finished") return "finished";
  return "published";
}

export function poolPresentationStatus(pool: Pick<PoolSummary, "closedAt" | "reopenedAt" | "event">, allPredictionsLocked = false): PresentationStatus {
  if (isPoolHistorical(pool)) return "finished";
  return allPredictionsLocked ? "locked" : "active";
}

export function isPoolHistorical(pool: Pick<PoolSummary, "closedAt" | "reopenedAt" | "event">): boolean {
  return pool.closedAt !== null || (pool.event.isHistorical && pool.reopenedAt === null);
}

export const presentationStatusLabel: Record<PresentationStatus, string> = {
  draft: "Rascunho",
  published: "Publicado",
  active: "Em andamento",
  locked: "Palpites encerrados",
  finished: "Encerrado",
};
