import { useState } from "react";
import { api } from "@/lib/api";
import type { PackageExportPreview } from "@/types";
import { Button } from "@/components/ui/button";

type Props = { eventId: string; slug: string; compact?: boolean };

export function EventPackageExport({ eventId, slug, compact }: Props) {
  const [preview, setPreview] = useState<PackageExportPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const download = async (path: string, fallback: string) => {
    setError("");
    try {
      const file = await api.download(path);
      const url = URL.createObjectURL(file.blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = file.filename ?? fallback;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Não foi possível exportar o evento.");
    }
  };
  const inspect = async () => {
    setBusy(true);
    setError("");
    try {
      setPreview(await api.get<PackageExportPreview>(`/custom/events/${eventId}/package/preview`));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Não foi possível verificar o pacote.");
    } finally {
      setBusy(false);
    }
  };
  const size = compact ? "sm" : undefined;
  return <>
    <Button size={size} variant="outline" className="w-full sm:w-auto" onClick={() => void download(`/custom/events/${eventId}/manifest`, `${slug}.json`)}>
      Exportar JSON
    </Button>
    <Button size={size} variant="outline" className="w-full sm:w-auto" disabled={busy} onClick={() => void inspect()}>
      Exportar pacote
    </Button>
    {error && <p className="col-span-2 w-full text-sm text-danger">{error}</p>}
    {preview && <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/45 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setPreview(null); }}>
      <div className="w-full max-w-lg rounded-[28px] border border-mint/20 bg-card p-6 shadow-2xl shadow-black/25 sm:p-7" role="dialog" aria-modal="true" aria-labelledby="package-export-title" onMouseDown={(event) => event.stopPropagation()}>
        <h2 id="package-export-title" className="text-xl">Conteúdo do pacote</h2>
        <p className="mt-2 text-ink-muted">{preview.assetCount} imagem(ns) interna(s) serão incluídas; {preview.externalImageCount} URL(s) externa(s) permanecerão como referência.</p>
        {preview.assetCount === 0 && <p className="mt-3 text-ink-muted">As imagens que aparecem apenas na revisão de trabalho ainda não existiam na versão publicada antes deste ajuste. Reenvie a capa e as imagens das opções para reparar eventos já afetados.</p>}
        {preview.externalImageCount > 0 && <div className="mt-4">
          <p className="text-ink-muted">URLs externas não são baixadas nem copiadas para o ZIP.</p>
          <ul className="mt-2 max-h-44 space-y-2 overflow-y-auto rounded-xl border border-mint/15 p-3 text-sm">
            {preview.externalImages.map((image) => <li key={`${image.question}-${image.optionLabel}-${image.url}`}>
              <p className="font-semibold">{image.question}{image.optionLabel ? ` · ${image.optionLabel}` : ""}</p>
              <a className="break-all text-mint-dark underline" href={image.url} target="_blank" rel="noreferrer">{image.url}</a>
            </li>)}
          </ul>
        </div>}
        <div className="mt-5 flex flex-wrap justify-end gap-2">
          {preview.assetCount === 0 && <a className="mr-auto self-center text-sm font-semibold text-mint-dark underline" href="#event-images" onClick={() => setPreview(null)}>Reenviar imagens</a>}
          <Button size="sm" variant="outline" onClick={() => setPreview(null)}>Cancelar</Button>
          <Button size="sm" onClick={() => void download(`/custom/events/${eventId}/package`, `${slug}.zip`)}>Baixar pacote</Button>
        </div>
      </div>
    </div>}
  </>;
}
