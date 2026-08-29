import type React from "react";
import { Card } from "@/components/ui/card";
import type { PoolReportStatus } from "@/types";

type AdminTab =
  | "overview"
  | "events"
  | "matches"
  | "predictions"
  | "scoring"
  | "users"
  | "pools"
  | "reports"
  | "audit"
  | "settings";

export const tabs: Array<{ id: AdminTab; label: string }> = [
  { id: "overview", label: "Resumo" },
  { id: "events", label: "Edições" },
  { id: "matches", label: "Jogos" },
  { id: "predictions", label: "Palpites" },
  { id: "scoring", label: "Pontuação" },
  { id: "users", label: "Usuários" },
  { id: "pools", label: "Bolões" },
  { id: "reports", label: "Denúncias" },
  { id: "audit", label: "Auditoria" },
  { id: "settings", label: "Configurações" },
];

export const reportCategoryLabels = {
  inappropriate_content: "Conteúdo inadequado",
  spam_or_fraud: "Spam ou fraude",
  harassment: "Assédio",
  other: "Outro",
} as const;

export const reportStatusLabels: Record<PoolReportStatus, string> = {
  open: "Aberta",
  reviewing: "Em análise",
  resolved: "Resolvida",
  dismissed: "Arquivada",
};
export const reportStatusOptions: PoolReportStatus[] = ["open", "reviewing", "resolved", "dismissed"];

export function MetricCard({
  icon,
  label,
  value,
  tone = "default",
}: {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  tone?: "default" | "danger" | "highlight";
}) {
  const toneClass =
    tone === "danger"
      ? "border-danger/30 bg-danger-bg"
      : tone === "highlight"
        ? "border-sky/40 bg-sky/15"
        : "border-mint/20 bg-card/80";

  return (
    <Card className={`border ${toneClass} p-4`}>
      <div className="flex items-center gap-3">
        <div className="rounded-full bg-card/80 p-2 text-mint-dark">{icon}</div>
        <div>
          <p className="text-xs uppercase tracking-[0.18em] text-ink-muted">{label}</p>
          <p className="mt-1 font-heading text-2xl font-semibold text-ink">{value}</p>
        </div>
      </div>
    </Card>
  );
}

export function TextArea(props: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={`min-h-28 w-full rounded-md border-2 border-mint/40 bg-card px-4 py-2.5 text-ink focus:border-mint-dark focus:outline-none focus:shadow-glow ${props.className ?? ""}`}
    />
  );
}

export function scoreField(value: number | null | undefined) {
  return value === null || value === undefined ? "" : String(value);
}

export function parseScore(value: string) {
  return value.trim() === "" ? 0 : Number.parseInt(value, 10) || 0;
}

