import { useEffect, useRef, useState } from "react";
import { ImageOff } from "lucide-react";
import { api } from "@/lib/api";
import type { AssetResponse } from "@/types";
import { Button } from "@/components/ui/button";

interface AssetUploadControlProps {
  label: string;
  currentUrl?: string | null;
  fallbackUrl?: string | null;
  uploadPath: string;
  removePath: string;
  disabled?: boolean;
  compact?: boolean;
  onChanged: (asset: AssetResponse | null) => void;
}

const MAX_BYTES = 10 * 1024 * 1024;

export function AssetUploadControl({
  label,
  currentUrl,
  fallbackUrl,
  uploadPath,
  removePath,
  disabled = false,
  compact = false,
  onChanged,
}: AssetUploadControlProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [usingFallback, setUsingFallback] = useState(false);
  const [imageFailed, setImageFailed] = useState(false);

  useEffect(() => () => {
    if (previewUrl?.startsWith("blob:")) URL.revokeObjectURL(previewUrl);
  }, [previewUrl]);

  const choose = async (file: File) => {
    setMessage("");
    if (file.size > MAX_BYTES) {
      setMessage("A imagem excede 10 MB.");
      return;
    }
    if (!["image/jpeg", "image/png", "image/webp"].includes(file.type)) {
      setMessage("Use JPEG, PNG ou WebP.");
      return;
    }
    const localUrl = URL.createObjectURL(file);
    setPreviewUrl(localUrl);
    setBusy(true);
    try {
      const asset = await api.upload<AssetResponse>(uploadPath, file);
      setPreviewUrl(null);
      setUsingFallback(false);
      setImageFailed(false);
      onChanged(asset);
      setMessage("Imagem atualizada");
    } catch (error) {
      setPreviewUrl(null);
      setMessage(error instanceof Error ? error.message : "Não foi possível enviar esta imagem.");
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    setMessage("");
    try {
      await api.post(removePath);
      setPreviewUrl(null);
      setUsingFallback(false);
      setImageFailed(false);
      onChanged(null);
      setMessage("Imagem removida");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Não foi possível remover esta imagem.");
    } finally {
      setBusy(false);
    }
  };

  const imageUrl = previewUrl ?? (usingFallback ? fallbackUrl : currentUrl);
  return (
    <div className={compact ? "flex items-center gap-2" : "rounded-xl border border-mint/20 bg-card/50 p-3"}>
      {imageUrl && !imageFailed ? (
        <img
          src={imageUrl}
          alt=""
          className={compact ? "h-12 w-12 rounded-lg object-cover" : "h-28 w-full rounded-lg object-cover"}
          loading="lazy"
          onError={() => {
            if (!previewUrl && fallbackUrl && !usingFallback) {
              setUsingFallback(true);
            } else {
              setImageFailed(true);
            }
          }}
        />
      ) : (
        <div
          className={compact
            ? "flex h-12 min-w-[4.5rem] flex-col items-center justify-center gap-1 rounded-lg bg-mint/10 px-2 text-[10px] font-medium leading-none text-ink-muted"
            : "flex h-28 w-full flex-col items-center justify-center gap-2 rounded-lg bg-mint/10 text-sm text-ink-muted"}
          title={`${label}: sem imagem selecionada`}
        >
          <ImageOff className={compact ? "h-4 w-4" : "h-5 w-5"} aria-hidden="true" />
          <span className="whitespace-nowrap">Sem imagem</span>
        </div>
      )}
      <div className={compact ? "flex flex-wrap gap-1" : "mt-2 flex flex-wrap items-center gap-2"}>
        <span className="sr-only">{label}</span>
        <input
          ref={inputRef}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          className="hidden"
          disabled={disabled || busy}
          onChange={(event) => {
            const file = event.target.files?.[0];
            event.target.value = "";
            if (file) void choose(file);
          }}
        />
        <Button size="sm" variant="secondary" disabled={disabled || busy} onClick={() => inputRef.current?.click()}>
          {busy ? "Enviando…" : imageUrl ? "Trocar" : "Enviar imagem"}
        </Button>
        {(imageUrl || currentUrl || fallbackUrl) && (
          <Button size="sm" variant="outline" disabled={disabled || busy} onClick={() => void remove()}>
            Remover
          </Button>
        )}
        {message && <span className="text-xs text-ink-muted">{message}</span>}
      </div>
    </div>
  );
}
