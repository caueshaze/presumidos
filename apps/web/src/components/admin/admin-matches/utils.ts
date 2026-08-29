export function parseScore(value: string) {
  return value.trim() === "" ? 0 : Number.parseInt(value, 10) || 0;
}

export function adminStatusLabel(status: string): string {
  switch (status) {
    case "scheduled": return "agendado";
    case "live": return "ao vivo";
    case "finished_pending": return "pendente de confirmação";
    case "finalized": return "finalizado";
    default: return status;
  }
}
