import { useRef, useState } from "react";
import { api } from "@/lib/api";
import { withAdminReauth } from "@/lib/adminReauth";
import { useReauth } from "@/hooks/queries";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";
import type { ManifestApplyResult, ManifestDiffEntry, ManifestPreview, PackageApplyResult, PackagePreview } from "@/types";

const MAX_BYTES = 2 * 1024 * 1024;

function actionLabel(action: ManifestPreview["action"]): string {
  switch (action) {
    case "create": return "Novo evento";
    case "noChange": return "Nenhuma alteração";
    case "safeUpdate": return "Nova revisão de conteúdo";
    case "conflict": return "Conflito bloqueado";
    default: return "Rejeitado";
  }
}

function DiffGroup({ title, entries }: { title: string; entries: ManifestDiffEntry[] }) {
  if (!entries.length) return null;
  return (
    <div className="mt-4">
      <h3 className="text-sm font-semibold uppercase tracking-[0.14em] text-ink-muted">{title}</h3>
      <div className="mt-2 space-y-2">
        {entries.map((entry, index) => (
          <div key={`${entry.path}-${index}`} className="flex flex-wrap justify-between gap-2 rounded-lg border border-mint/15 bg-card/60 px-3 py-2 text-sm">
            <span className="font-medium text-ink">{entry.path}</span>
            <span className="text-ink-muted">{entry.change}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function AdminManifestPanel({ onApplied }: { onApplied?: () => void }) {
  const input = useRef<HTMLInputElement>(null);
  const reauth = useReauth();
  const [file, setFile] = useState<File | null>(null);
  const [content, setContent] = useState("");
  const [preview, setPreview] = useState<ManifestPreview | null>(null);
  const [packagePreview, setPackagePreview] = useState<PackagePreview | null>(null);
  const [success, setSuccess] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  const validateAndPreview = async () => {
    if (!file) { setError("Selecione um manifesto ou pacote."); return; }
    const isPackage = file.name.toLowerCase().endsWith(".zip");
    if (!isPackage && !file.name.toLowerCase().endsWith(".json")) { setError("Apenas arquivos .json ou .zip são aceitos."); return; }
    if (!isPackage && file.size > MAX_BYTES) { setError("O manifesto excede o limite de 2 MB."); return; }
    setBusy(true); setError(""); setSuccess("");
    try {
      if (isPackage) {
        const next = await api.upload<PackagePreview>("/admin/events/import/package/preview", file);
        setPackagePreview(next); setPreview(next.manifest); setContent("");
      } else {
        const text = await file.text();
        const next = await api.post<ManifestPreview>("/admin/events/import/preview", { content: text, filename: file.name });
        setPackagePreview(null); setContent(text); setPreview(next);
      }
    } catch (err) {
      setPreview(null); setError(err instanceof Error ? err.message : "Não foi possível validar o manifesto.");
    } finally { setBusy(false); }
  };

  const apply = async () => {
    if (!preview || preview.action === "conflict" || preview.action === "rejected" || preview.action === "noChange") return;
    if (!confirming) { setConfirming(true); return; }
    setBusy(true); setError(""); setSuccess("");
    try {
      const result = await withAdminReauth(
        async () => {
          // Evita transmitir um ZIP quando a confirmação administrativa já expirou.
          // A rota de aplicação confere novamente para preservar a segurança.
          await api.post<void>("/admin/reauth/verify");
          return packagePreview && file
            ? api.upload<PackageApplyResult>("/admin/events/import/package/apply", file, { baseFingerprint: preview.baseFingerprint }).then((value) => value.result)
            : api.post<ManifestApplyResult>("/admin/events/import/apply", { content, baseFingerprint: preview.baseFingerprint, filename: file?.name });
        },
        (password) => reauth.mutateAsync(password),
      );
      setSuccess(result.state === "working" ? "Revisão criada. O conteúdo público continua na versão anterior até a publicação." : "Versão publicada.");
      onApplied?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : "A importação falhou; nada foi alterado.");
    } finally { setBusy(false); setConfirming(false); }
  };

  return (
    <Card className="border border-sky/25 bg-sky/5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-xl">Importar JSON ou pacote do evento</h2>
          <p className="mt-1 text-sm text-ink-muted">O upload apenas valida e prepara o preview. Nenhuma alteração é aplicada nessa etapa.</p>
        </div>
        <Button variant="outline" onClick={() => input.current?.click()} disabled={busy}>Selecionar JSON ou pacote</Button>
        <input ref={input} hidden type="file" accept=".json,.zip,application/json,application/zip" aria-label="Manifesto ou pacote" onChange={(event) => { setFile(event.target.files?.[0] ?? null); setPreview(null); setPackagePreview(null); setSuccess(""); }} />
      </div>
      {file && <p className="mt-3 text-sm font-medium text-ink">Arquivo: {file.name}</p>}
      <div className="mt-3 flex flex-wrap gap-2">
        <Button onClick={() => void validateAndPreview()} disabled={!file || busy}>{busy ? "Processando…" : "Validar e visualizar"}</Button>
        {preview && <span className="rounded-pill bg-card px-3 py-2 text-sm font-semibold text-mint-dark">{actionLabel(preview.action)}</span>}
      </div>
      {error && <div className="mt-3"><ErrorBanner>{error}</ErrorBanner></div>}
      {success && <p className="mt-3 text-sm font-semibold text-mint-dark">{success}</p>}
      {preview && (
        <div className="mt-5 border-t border-mint/15 pt-4">
          <h3 className="text-lg">{preview.name}</h3>
          <p className="mt-1 text-sm text-ink-muted">{preview.slug} · schemaVersion {preview.schemaVersion}</p>
          <div className="mt-3 grid gap-2 text-sm sm:grid-cols-3">
            <span>Perguntas: <strong>{preview.itemCount}</strong></span>
            <span>Opções: <strong>{preview.optionCount}</strong></span>
            <span>Links: <strong>{preview.linkCount}</strong></span>
          </div>
          {packagePreview && <div className="mt-3 rounded-lg border border-mint/15 bg-card/60 px-3 py-2 text-sm"><strong>Assets:</strong> {packagePreview.assetCount} no pacote · {packagePreview.existingAssetCount} já existentes · {packagePreview.addedAssetCount} serão adicionados</div>}
          <DiffGroup title="Alterações da próxima revisão" entries={preview.safeChanges} />
          <DiffGroup title="Conflitos que precisam de correção" entries={preview.blockedChanges} />
          {preview.action === "conflict" && <p className="mt-4 rounded-lg bg-danger-bg px-3 py-2 text-sm font-semibold text-danger">Há um conflito de base, identidade ou validação. A revisão não foi criada.</p>}
          {preview.action === "noChange" && <p className="mt-4 rounded-lg bg-card px-3 py-2 text-sm text-ink-muted">O evento já está semanticamente igual ao manifesto.</p>}
          <div className="mt-4 flex justify-end">
            {!confirming ? (
              <Button onClick={() => void apply()} disabled={busy || !["create", "safeUpdate"].includes(preview.action)}>Aplicar manifesto</Button>
            ) : (
              <div className="flex flex-wrap justify-end gap-2">
                <Button variant="outline" onClick={() => setConfirming(false)} disabled={busy}>Cancelar</Button>
                <Button onClick={() => void apply()} disabled={busy}>{busy ? "Aplicando…" : "Confirmar aplicação"}</Button>
              </div>
            )}
          </div>
        </div>
      )}
    </Card>
  );
}
