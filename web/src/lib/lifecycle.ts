import type { EventSummary } from "@/types";

/** A única tradução de lifecycle de domínio para linguagem da interface. */
export type PresentationStatus = "draft" | "published" | "active" | "locked" | "finished";

export function eventPresentationStatus(event: Pick<EventSummary, "status" | "isHistorical">): PresentationStatus {
  if (event.status === "draft") return "draft";
  if (event.isHistorical || event.status === "finished") return "finished";
  return "published";
}

export function poolPresentationStatus(event: Pick<EventSummary, "status" | "isHistorical">, allPredictionsLocked = false): PresentationStatus {
  if (event.isHistorical || event.status === "finished") return "finished";
  return allPredictionsLocked ? "locked" : "active";
}

export const presentationStatusLabel: Record<PresentationStatus, string> = {
  draft: "Rascunho",
  published: "Publicado",
  active: "Em andamento",
  locked: "Palpites encerrados",
  finished: "Encerrado",
};
