import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Check, ChevronDown, Image as ImageIcon, Pencil, Trophy, X } from "lucide-react";
import { api } from "@/lib/api";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner, Select } from "@/components/ui/field";
import { AssetUploadControl } from "@/components/AssetUploadControl";
import { SingleChoicePredictionCard } from "@/components/SingleChoicePredictionCard";
import { NumericPredictionCard } from "@/components/NumericPredictionCard";
import { MultipleChoicePredictionCard } from "@/components/MultipleChoicePredictionCard";
import type { CustomQuestion, OptionLink } from "@/types";
import { useAuth } from "@/hooks/useAuth";

type Option = { id: string; label: string; imageUrl?: string | null; imageAssetUrl?: string | null; links?: OptionLink[] };
type Item = {
  id: string;
  kind: "single_choice" | "numeric" | "multiple_choice";
  title: string;
  lockAt: string;
  revealAt: string;
  correctOptionId: string | null;
  options: Option[];
  decimalPlaces?: number;
  unitLabel?: string | null;
  minValue?: string | null;
  maxValue?: string | null;
  resultValue?: string | null;
  minSelections?: number;
  maxSelections?: number | null;
};
type Draft = {
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
};

export function EventBuilderPage() {
  const { eventId } = useParams();
  const navigate = useNavigate();
  const { isAdmin, user } = useAuth();
  const [draft, setDraft] = useState<Draft | null>(null);
  const [name, setName] = useState("");
  const [startsAt, setStartsAt] = useState("");
  const [endsAt, setEndsAt] = useState("");
  const [description, setDescription] = useState("");
  const [coverUrl, setCoverUrl] = useState("");
  const [externalUrl, setExternalUrl] = useState("");
  const [title, setTitle] = useState("");
  const [itemKind, setItemKind] = useState<
    "single_choice" | "numeric" | "multiple_choice"
  >("single_choice");
  const [decimalPlaces, setDecimalPlaces] = useState("0");
  const [unitLabel, setUnitLabel] = useState("");
  const [minValue, setMinValue] = useState("");
  const [maxValue, setMaxValue] = useState("");
  const [minSelections, setMinSelections] = useState("1");
  const [maxSelections, setMaxSelections] = useState("");
  const [lockAt, setLockAt] = useState("");
  const [revealAt, setRevealAt] = useState("");
  const [labels, setLabels] = useState<Record<string, string>>({});
  const [mediaDrafts, setMediaDrafts] = useState<Record<string, { imageUrl: string; links: OptionLink[] }>>({});
  const [openMediaOptionId, setOpenMediaOptionId] = useState<string | null>(null);
  const [editingOptionId, setEditingOptionId] = useState<string | null>(null);
  const [optionLabelDraft, setOptionLabelDraft] = useState("");
  const [results, setResults] = useState<Record<string, string>>({});
  const [multipleResults, setMultipleResults] = useState<
    Record<string, string[]>
  >({});
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const load = async (id: string) => {
    try {
      const next = await api.get<Draft>(`/custom/events/${id}/draft`);
      setDraft(next);
      setName(next.event.name);
      setStartsAt(next.event.startsAt?.slice(0, 16) ?? "");
      setEndsAt(next.event.endsAt?.slice(0, 16) ?? "");
      setDescription(next.event.description ?? "");
      setCoverUrl(next.event.coverUrl ?? "");
      setExternalUrl(next.event.externalUrl ?? "");
      setMediaDrafts(Object.fromEntries(next.items.flatMap((item) => item.options.map((option) => [option.id, {
        imageUrl: option.imageUrl ?? "",
        links: option.links ?? [],
      }]))));
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Não foi possível carregar o evento.",
      );
    }
  };
  useEffect(() => {
    if (eventId) void load(eventId);
  }, [eventId]);
  const create = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      const event = await api.post<{ id: string }>("/custom/events", {
        name,
        startsAt: startsAt ? new Date(startsAt).toISOString() : null,
        endsAt: endsAt ? new Date(endsAt).toISOString() : null,
      });
      navigate(`/events/${event.id}`);
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Não foi possível criar o rascunho.",
      );
    } finally {
      setBusy(false);
    }
  };
  const addItem = async (e: FormEvent) => {
    e.preventDefault();
    if (!draft) return;
    setBusy(true);
    try {
      const path =
        itemKind === "numeric"
          ? "/numeric"
          : itemKind === "multiple_choice"
            ? "/multiple-choice"
            : "";
      const body =
        itemKind === "numeric"
          ? {
              title,
              lockAt,
              revealAt,
              decimalPlaces: Number(decimalPlaces),
              unitLabel: unitLabel || null,
              minValue: minValue || null,
              maxValue: maxValue || null,
            }
          : itemKind === "multiple_choice"
            ? {
                title,
                lockAt,
                revealAt,
                minSelections: Number(minSelections),
                maxSelections: maxSelections ? Number(maxSelections) : null,
              }
            : { title, lockAt, revealAt };
      await api.post(`/custom/events/${draft.event.id}/items${path}`, body);
      setTitle("");
      setUnitLabel("");
      setMinValue("");
      setMaxValue("");
      setMinSelections("1");
      setMaxSelections("");
      await load(draft.event.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Falha ao adicionar pergunta.");
    } finally {
      setBusy(false);
    }
  };
  const action = async (path: string, body?: unknown): Promise<boolean> => {
    if (!draft) return false;
    setBusy(true);
    try {
      await api.post(path, body);
      await load(draft.event.id);
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Operação recusada.");
      return false;
    } finally {
      setBusy(false);
    }
  };
  const saveMetadata = async () => {
    if (!draft) return;
    await action(`/custom/events/${draft.event.id}/update`, {
      name,
      startsAt: startsAt ? new Date(startsAt).toISOString() : null,
      endsAt: endsAt ? new Date(endsAt).toISOString() : null,
      description: description || null,
      coverUrl: coverUrl || null,
      externalUrl: externalUrl || null,
    });
  };
  const editItem = async (item: Item) => {
    const nextTitle = window.prompt("Pergunta", item.title);
    const nextLock = window.prompt("Fecha em (ISO)", item.lockAt);
    const nextReveal = window.prompt("Revela em (ISO)", item.revealAt);
    if (nextTitle && nextLock && nextReveal && draft)
      await action(`/custom/events/${draft.event.id}/items/${item.id}/update`, {
        title: nextTitle,
        lockAt: nextLock,
        revealAt: nextReveal,
      });
  };
  const startOptionEdit = (option: Option) => {
    setError("");
    setEditingOptionId(option.id);
    setOptionLabelDraft(option.label);
  };
  const cancelOptionEdit = () => {
    setEditingOptionId(null);
    setOptionLabelDraft("");
  };
  const saveOptionLabel = async (item: Item, option: Option) => {
    const label = optionLabelDraft.trim();
    if (!label) {
      setError("O nome da opção não pode ficar vazio.");
      return;
    }
    if (!draft) return;
    const saved = await action(
      `/custom/events/${draft.event.id}/items/${item.id}/options/${option.id}/update`,
      { label },
    );
    if (saved) cancelOptionEdit();
  };
  const saveOptionMedia = async (item: Item, option: Option) => {
    if (!draft) return;
    const media = mediaDrafts[option.id] ?? { imageUrl: option.imageUrl ?? "", links: option.links ?? [] };
    await action(`/custom/events/${draft.event.id}/items/${item.id}/options/${option.id}/media`, {
      imageUrl: media.imageUrl || null,
      links: media.links.filter((link) => link.url.trim()).map((link, sortOrder) => ({ ...link, sortOrder })),
    });
  };
  const downloadEventFile = async (kind: "manifest" | "package") => {
    if (!draft) return;
    try {
      const download = await api.download(kind === "manifest" ? `/custom/events/${draft.event.id}/manifest` : `/custom/events/${draft.event.id}/package`);
      const url = URL.createObjectURL(download.blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = download.filename ?? `${draft.event.name}.${kind === "manifest" ? "json" : "zip"}`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Não foi possível exportar o evento.");
    }
  };
  const deleteDraft = async () => {
    if (!draft || !window.confirm(`Apagar o rascunho "${draft.event.name}"?`))
      return;
    setBusy(true);
    try {
      await api.post(`/custom/events/${draft.event.id}/delete`);
      navigate("/dashboard");
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Não foi possível apagar o rascunho.",
      );
    } finally {
      setBusy(false);
    }
  };
  if (!eventId)
    return (
      <PageShell>
        <Button variant="link" size="sm" onClick={() => navigate("/events")}>← Voltar aos eventos</Button>
        <h1 className="mt-3 text-3xl">Criar evento</h1>
        <Card className="mt-5">
          <form onSubmit={create} className="flex flex-col gap-3">
            <label>
              Nome do evento
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </label>
            <label>
              Data inicial
              <Input
                type="datetime-local"
                value={startsAt}
                onChange={(e) => setStartsAt(e.target.value)}
              />
            </label>
            <label>
              Data final
              <Input
                type="datetime-local"
                value={endsAt}
                onChange={(e) => setEndsAt(e.target.value)}
              />
            </label>
            <Button disabled={busy}>Criar rascunho</Button>
          </form>
          {error && <ErrorBanner>{error}</ErrorBanner>}
        </Card>
      </PageShell>
    );
  if (!draft)
    return (
      <PageShell>
        <Button variant="link" size="sm" onClick={() => navigate("/events")}>← Voltar aos eventos</Button>
        <p className="mt-3">Carregando evento…</p>
        {error && <ErrorBanner>{error}</ErrorBanner>}
      </PageShell>
    );
  const editable = draft.event.status === "draft";
  const metadataEditable = editable || isAdmin;
  const mediaEditable = editable || isAdmin || draft.event.createdBy === user?.id;
  const hasInternalAssets = Boolean(
    draft.event.coverAssetId ||
      draft.items.some((item) => item.options.some((option) => option.imageAssetUrl)),
  );
  return (
    <PageShell>
      <Button variant="link" size="sm" onClick={() => navigate("/events")}>← Voltar aos eventos</Button>
      <div className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-3xl">{draft.event.name}</h1>
          <p className="text-ink-muted">
            {editable
              ? "Rascunho privado · Escolha única, múltipla escolha ou número"
              : "Publicado · estrutura imutável"}
          </p>
        </div>
        {editable ? (
          <div className="flex flex-wrap justify-end gap-2">
            <Button size="sm" variant="outline" onClick={() => void downloadEventFile("manifest")}>Exportar JSON</Button>
            <Button size="sm" variant="outline" onClick={() => void downloadEventFile("package")}>Exportar pacote</Button>
            <Button
              variant="outline"
              className="text-danger"
              disabled={busy}
              onClick={() => void deleteDraft()}
            >
              Apagar rascunho
            </Button>
            <Button
              disabled={busy}
              onClick={() => action(`/custom/events/${draft.event.id}/publish`)}
            >
              Publicar
            </Button>
          </div>
        ) : (
          <div className="flex flex-wrap justify-end gap-2">
            <Button size="sm" variant="outline" onClick={() => void downloadEventFile("manifest")}>Exportar JSON</Button>
            <Button size="sm" variant="outline" onClick={() => void downloadEventFile("package")}>Exportar pacote</Button>
            <Button onClick={() => navigate(`/dashboard?eventId=${draft.event.id}`)}>Criar bolão</Button>
          </div>
        )}
      </div>
      {error && (
        <div className="mt-3">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}
      {hasInternalAssets && (
        <p className="mt-3 rounded-lg border border-sky/25 bg-sky/5 px-3 py-2 text-sm text-ink-muted">
          Este evento usa arquivos locais. Para promovê-lo para outro ambiente,
          exporte o pacote completo; o JSON contém apenas as referências por hash.
        </p>
      )}
      <Card className="mt-5">
        <div className="flex flex-col gap-3">
          <label>
            Nome do evento
            <Input
              disabled={!metadataEditable}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label>
            Data inicial
            <Input
              disabled={editable ? false : true}
              type="datetime-local"
              value={startsAt}
              onChange={(e) => setStartsAt(e.target.value)}
            />
          </label>
          <label>
            Data final
            <Input
              disabled={editable ? false : true}
              type="datetime-local"
              value={endsAt}
              onChange={(e) => setEndsAt(e.target.value)}
            />
          </label>
          <label>
            Descrição <span className="text-ink-muted">(opcional)</span>
            <textarea disabled={!metadataEditable} value={description} onChange={(e) => setDescription(e.target.value)} maxLength={1200} className="mt-1 min-h-24 w-full rounded-xl border border-mint/25 bg-card/60 px-3 py-2" />
          </label>
          <label>
            Capa do evento
            {draft && (
              <AssetUploadControl
                label="Capa do evento"
                currentUrl={draft.event.coverAssetUrl ?? coverUrl}
                fallbackUrl={draft.event.coverAssetUrl ? coverUrl : undefined}
                uploadPath={`/custom/events/${draft.event.id}/cover`}
                removePath={`/custom/events/${draft.event.id}/cover/remove`}
                disabled={!mediaEditable}
                onChanged={(asset) => setDraft((current) => current ? { ...current, event: { ...current.event, coverAssetId: asset?.assetId ?? null, coverAssetUrl: asset?.url ?? null } } : current)}
              />
            )}
            <span className="mt-2 block text-xs text-ink-muted">URL externa (opcional)</span>
            <Input disabled={!metadataEditable} type="url" placeholder="https://..." value={coverUrl} onChange={(e) => setCoverUrl(e.target.value)} />
          </label>
          <label>
            Site oficial <span className="text-ink-muted">(opcional)</span>
            <Input disabled={!metadataEditable} type="url" placeholder="https://..." value={externalUrl} onChange={(e) => setExternalUrl(e.target.value)} />
          </label>
          {metadataEditable && (
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => void saveMetadata()}
            >
              Salvar informações
            </Button>
          )}
        </div>
      </Card>
      {editable && (
        <Card className="mt-5">
          <form onSubmit={addItem} className="flex flex-col gap-3">
            <h2 className="text-xl">Adicionar pergunta</h2>
            <label>
              Tipo da pergunta
              <select
                aria-label="Tipo da pergunta"
                value={itemKind}
                onChange={(e) =>
                  setItemKind(
                    e.target.value as
                      "single_choice" | "multiple_choice" | "numeric",
                  )
                }
              >
                <option value="single_choice">Escolha única</option>
                <option value="multiple_choice">Múltipla escolha</option>
                <option value="numeric">Número</option>
              </select>
            </label>
            <label>
              Pergunta
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
              />
            </label>
            {itemKind === "multiple_choice" && (
              <>
                <label>
                  Mínimo de escolhas
                  <Input
                    type="number"
                    min="1"
                    value={minSelections}
                    onChange={(e) => setMinSelections(e.target.value)}
                    required
                  />
                </label>
                <label>
                  Máximo de escolhas (opcional)
                  <Input
                    type="number"
                    min="1"
                    value={maxSelections}
                    onChange={(e) => setMaxSelections(e.target.value)}
                  />
                </label>
              </>
            )}
            {itemKind === "numeric" && (
              <>
                <label>
                  Unidade (opcional)
                  <Input
                    value={unitLabel}
                    onChange={(e) => setUnitLabel(e.target.value)}
                  />
                </label>
                <label>
                  Casas decimais
                  <Input
                    type="number"
                    min="0"
                    max="6"
                    value={decimalPlaces}
                    onChange={(e) => setDecimalPlaces(e.target.value)}
                    required
                  />
                </label>
                <label>
                  Valor mínimo (opcional)
                  <Input
                    inputMode="decimal"
                    value={minValue}
                    onChange={(e) => setMinValue(e.target.value)}
                  />
                </label>
                <label>
                  Valor máximo (opcional)
                  <Input
                    inputMode="decimal"
                    value={maxValue}
                    onChange={(e) => setMaxValue(e.target.value)}
                  />
                </label>
              </>
            )}
            <label>
              Fecha em (ISO)
              <Input
                value={lockAt}
                onChange={(e) => setLockAt(e.target.value)}
                placeholder="2026-12-31T18:00:00Z"
                required
              />
            </label>
            <label>
              Revela em (ISO)
              <Input
                value={revealAt}
                onChange={(e) => setRevealAt(e.target.value)}
                placeholder="2026-12-31T20:00:00Z"
                required
              />
            </label>
            <Button disabled={busy}>Adicionar pergunta</Button>
          </form>
        </Card>
      )}
      <div className="mt-4 flex flex-col gap-4">
        {draft.items.map((item, index) => (
          <Card key={item.id}>
            <div className="flex justify-between gap-2">
              <div>
                <h2 className="text-xl">{item.title}</h2>
                <p className="text-sm text-ink-muted">
                  {item.kind === "numeric"
                    ? `Número${item.unitLabel ? ` · ${item.unitLabel}` : ""} · ${item.decimalPlaces ?? 0} casas`
                    : item.kind === "multiple_choice"
                      ? `Múltipla escolha · ${item.minSelections ?? 1}–${item.maxSelections ?? item.options.length} opções`
                      : "Escolha única"}{" "}
                  · Fecha: {item.lockAt}
                </p>
              </div>
              {editable && (
                <div className="flex gap-1">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={busy}
                    onClick={() => void editItem(item)}
                  >
                    Editar
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={busy || index === 0}
                    onClick={() =>
                      action(
                        `/custom/events/${draft.event.id}/items/${item.id}/move`,
                        { direction: -1 },
                      )
                    }
                  >
                    ↑
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={busy || index === draft.items.length - 1}
                    onClick={() =>
                      action(
                        `/custom/events/${draft.event.id}/items/${item.id}/move`,
                        { direction: 1 },
                      )
                    }
                  >
                    ↓
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-danger"
                    disabled={busy}
                    onClick={() =>
                      action(
                        `/custom/events/${draft.event.id}/items/${item.id}/delete`,
                      )
                    }
                  >
                    Remover
                  </Button>
                </div>
              )}
            </div>
            {(item.kind === "single_choice" ||
              item.kind === "multiple_choice") && (
              <>
                <ol className="mt-4 space-y-3 pl-0">
                  {item.options.map((o, optionIndex) => (
                    <li key={o.id} className="flex items-start gap-3 rounded-xl border border-mint/20 bg-card/35 p-3 shadow-sm transition-colors hover:border-mint/35 hover:bg-card/55">
                      <span className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-mint/15 text-xs font-bold text-mint-dark">
                        {optionIndex + 1}
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          {editingOptionId === o.id ? (
                            <div className="flex min-w-0 max-w-xl flex-1 items-center gap-2">
                              <Input
                                autoFocus
                                aria-label={`Nome da opção ${o.label}`}
                                value={optionLabelDraft}
                                onChange={(event) => setOptionLabelDraft(event.target.value)}
                                onKeyDown={(event) => {
                                  if (event.key === "Enter") void saveOptionLabel(item, o);
                                  if (event.key === "Escape") cancelOptionEdit();
                                }}
                              />
                              <Button
                                type="button"
                                size="sm"
                                aria-label="Salvar nome da opção"
                                className="rounded-lg px-2.5"
                                disabled={busy}
                                onClick={() => void saveOptionLabel(item, o)}
                              >
                                <Check className="h-4 w-4" />
                              </Button>
                              <Button
                                type="button"
                                size="sm"
                                variant="outline"
                                aria-label="Cancelar edição do nome"
                                className="rounded-lg px-2.5"
                                disabled={busy}
                                onClick={cancelOptionEdit}
                              >
                                <X className="h-4 w-4" />
                              </Button>
                            </div>
                          ) : (
                            <span className="font-medium text-ink">{o.label}</span>
                          )}
                        {mediaEditable && (() => {
                          const media = mediaDrafts[o.id] ?? { imageUrl: o.imageUrl ?? "", links: o.links ?? [] };
                          const mediaOpen = openMediaOptionId === o.id;
                          const hasMedia = Boolean(o.imageAssetUrl || o.imageUrl || o.links?.length);
                          return (
                            <>
                              <button
                                type="button"
                                title={hasMedia ? "Editar mídia configurada" : "Adicionar mídia opcional"}
                                aria-label={hasMedia ? "Editar mídia configurada" : "Adicionar mídia opcional"}
                                aria-expanded={mediaOpen}
                                className={`inline-flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-xs font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-mint-dark/40 ${mediaOpen ? "border-mint-dark bg-mint/25 text-mint-dark shadow-glow" : hasMedia ? "border-mint-dark/40 bg-mint/15 text-mint-dark hover:bg-mint/25" : "border-mint/20 bg-card/70 text-ink-muted hover:border-mint/40 hover:bg-mint/10 hover:text-mint-dark"}`}
                                onClick={() => setOpenMediaOptionId((current) => current === o.id ? null : o.id)}
                              >
                                <ImageIcon className="h-4 w-4" />
                                <span>Mídia</span>
                                <ChevronDown className={`h-3.5 w-3.5 transition-transform duration-200 ${mediaOpen ? "rotate-180" : ""}`} />
                              </button>
                              {mediaOpen && (
                                <div className="basis-full rounded-xl border border-mint/15 bg-card/40 p-3">
                                  <AssetUploadControl
                                    label={`Imagem da opção ${o.label}`}
                                    currentUrl={o.imageAssetUrl ?? o.imageUrl}
                                    fallbackUrl={o.imageAssetUrl ? o.imageUrl : undefined}
                                    uploadPath={`/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/image`}
                                    removePath={`/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/image/remove`}
                                    disabled={!mediaEditable}
                                    compact
                                    onChanged={() => void load(draft.event.id)}
                                  />
                                  <p className="mt-2 text-xs text-ink-muted">URL externa (opcional)</p>
                                  <Input
                                    aria-label={`Imagem da opção ${o.label}`}
                                    className="mt-2"
                                    placeholder="URL da imagem (https://…)"
                                    value={media.imageUrl}
                                    onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, imageUrl: event.target.value } }))}
                                  />
                                  {media.links.map((link, linkIndex) => (
                                    <div className="mt-2 grid gap-2 sm:grid-cols-[8rem_1fr_1fr_auto]" key={`${o.id}-link-${linkIndex}`}>
                                      <Input aria-label={`Tipo do link ${o.label} ${linkIndex + 1}`} value={link.kind} placeholder="tipo" onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, kind: event.target.value as OptionLink["kind"] } : entry) } }))} />
                                      <Input aria-label={`Rótulo do link ${o.label} ${linkIndex + 1}`} value={link.label} placeholder="rótulo" onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, label: event.target.value } : entry) } }))} />
                                      <Input aria-label={`URL do link ${o.label} ${linkIndex + 1}`} value={link.url} placeholder="https://…" onChange={(event) => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.map((entry, index) => index === linkIndex ? { ...entry, url: event.target.value } : entry) } }))} />
                                      <Button size="sm" variant="outline" onClick={() => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: media.links.filter((_, index) => index !== linkIndex) } }))}>Remover</Button>
                                    </div>
                                  ))}
                                  <div className="mt-2 flex flex-wrap gap-2">
                                    <Button size="sm" variant="outline" onClick={() => setMediaDrafts((current) => ({ ...current, [o.id]: { ...media, links: [...media.links, { kind: "other", label: "", url: "", sortOrder: media.links.length }] } }))}>Adicionar link</Button>
                                    <Button size="sm" variant="secondary" disabled={busy} onClick={() => void saveOptionMedia(item, o)}>Salvar mídia</Button>
                                  </div>
                                </div>
                              )}
                            </>
                          );
                        })()}
                        </div>
                      </div>
                      {(editable || mediaEditable) && (
                        <span className="flex gap-1">
                          <Button
                            size="sm"
                            variant="outline"
                            className="rounded-lg border-transparent bg-transparent px-2 text-ink-muted shadow-none hover:border-transparent hover:bg-mint/10 hover:text-ink"
                            onClick={() => startOptionEdit(o)}
                          >
                            <Pencil className="h-3.5 w-3.5" />
                            Editar
                          </Button>
                          {editable && <Button
                            size="sm"
                            variant="outline"
                            disabled={optionIndex === 0}
                            onClick={() =>
                              action(
                                `/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/move`,
                                { direction: -1 },
                              )
                            }
                          >
                            ↑
                          </Button>}
                          {editable && <Button
                            size="sm"
                            variant="outline"
                            disabled={optionIndex === item.options.length - 1}
                            onClick={() =>
                              action(
                                `/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/move`,
                                { direction: 1 },
                              )
                            }
                          >
                            ↓
                          </Button>}
                          {editable && <Button
                            size="sm"
                            variant="outline"
                            className="text-danger"
                            onClick={() =>
                              action(
                                `/custom/events/${draft.event.id}/items/${item.id}/options/${o.id}/delete`,
                              )
                            }
                          >
                            Remover
                          </Button>}
                        </span>
                      )}
                    </li>
                  ))}
                </ol>
                {editable && (
                  <div className="mt-3 flex gap-2">
                    <Input
                      aria-label={`Nova opção para ${item.title}`}
                      value={labels[item.id] ?? ""}
                      onChange={(e) =>
                        setLabels((v) => ({ ...v, [item.id]: e.target.value }))
                      }
                    />
                    <Button
                      onClick={() => {
                        const label = labels[item.id];
                        if (label) {
                          void action(
                            `/custom/events/${draft.event.id}/items/${item.id}/options`,
                            { label },
                          );
                          setLabels((v) => ({ ...v, [item.id]: "" }));
                        }
                      }}
                    >
                      Adicionar opção
                    </Button>
                  </div>
                )}
                {!editable && item.kind === "single_choice" && (
                  <div className="mt-5 rounded-xl border border-sky/25 bg-sky/5 p-4">
                    <div className="flex items-start gap-3">
                      <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-sky/15 text-sky-dark">
                        <Trophy className="h-4 w-4" />
                      </span>
                      <div>
                        <p className="font-semibold text-ink">Resultado oficial</p>
                        <p className="mt-0.5 text-xs text-ink-muted">Selecione o vencedor desta pergunta para fechar a apuração.</p>
                      </div>
                    </div>
                    <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:items-center">
                    <Select
                      aria-label={`Resultado oficial: ${item.title}`}
                      value={results[item.id] ?? item.correctOptionId ?? ""}
                      onChange={(e) =>
                        setResults((v) => ({ ...v, [item.id]: e.target.value }))
                      }
                    >
                      <option value="">Selecione o vencedor</option>
                      {item.options.map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.label}
                        </option>
                      ))}
                    </Select>
                    <Button
                      size="sm"
                      className="w-full sm:w-auto"
                      disabled={!(results[item.id] ?? item.correctOptionId)}
                      onClick={() =>
                        action(`/admin/custom/questions/${item.id}/result`, {
                          optionId: results[item.id] ?? item.correctOptionId,
                        })
                      }
                    >
                      <Check className="h-4 w-4" />
                      Salvar resultado
                    </Button>
                    </div>
                  </div>
                )}
                {!editable && item.kind === "multiple_choice" && (
                  <div className="mt-3 space-y-2">
                    <p className="text-sm font-semibold">Resultado oficial</p>
                    {item.options.map((option) => {
                      const selected = multipleResults[item.id] ?? [];
                      return (
                        <label
                          key={option.id}
                          className="flex items-center gap-2"
                        >
                          <input
                            type="checkbox"
                            checked={selected.includes(option.id)}
                            onChange={() =>
                              setMultipleResults((all) => ({
                                ...all,
                                [item.id]: selected.includes(option.id)
                                  ? selected.filter((id) => id !== option.id)
                                  : [...selected, option.id],
                              }))
                            }
                          />
                          {option.label}
                        </label>
                      );
                    })}
                    <Button
                      size="sm"
                      disabled={
                        (multipleResults[item.id]?.length ?? 0) <
                        (item.minSelections ?? 1)
                      }
                      onClick={() =>
                        action(
                          `/admin/custom/multiple-choice/${item.id}/result`,
                          { optionIds: multipleResults[item.id] ?? [] },
                        )
                      }
                    >
                      Salvar resultado
                    </Button>
                  </div>
                )}
              </>
            )}
            {item.kind === "numeric" && !editable && (
              <div className="mt-3 flex gap-2">
                <Input
                  inputMode="decimal"
                  aria-label={`Resultado oficial: ${item.title}`}
                  value={results[item.id] ?? item.resultValue ?? ""}
                  onChange={(e) =>
                    setResults((v) => ({ ...v, [item.id]: e.target.value }))
                  }
                />
                <Button
                  size="sm"
                  disabled={!(results[item.id] ?? item.resultValue)}
                  onClick={() =>
                    action(`/admin/custom/numeric/${item.id}/result`, {
                      value: results[item.id] ?? item.resultValue,
                    })
                  }
                >
                  Salvar resultado
                </Button>
              </div>
            )}
          </Card>
        ))}
        {draft.items.length > 0 && (
          <section>
            <h2 className="mb-3 text-2xl">Prévia</h2>
            {draft.items.map((item, index) =>
              item.kind === "numeric" ? (
                <NumericPredictionCard
                  key={`preview-${item.id}`}
                  preview
                  poolId="preview"
                  index={index}
                  question={{
                    itemId: item.id,
                    kind: "numeric",
                    title: item.title,
                    lockAt: item.lockAt,
                    revealAt: item.revealAt,
                    sortOrder: index,
                    status: "open",
                    currentOptionId: null,
                    correctOptionId: null,
                    correctPoints: 0,
                    incorrectPoints: 0,
                    options: [],
                    decimalPlaces: item.decimalPlaces,
                    unitLabel: item.unitLabel,
                    minValue: item.minValue,
                    maxValue: item.maxValue,
                    exactPoints: 1,
                  }}
                />
              ) : item.kind === "multiple_choice" ? (
                <MultipleChoicePredictionCard
                  key={`preview-${item.id}`}
                  preview
                  poolId="preview"
                  index={index}
                  question={{
                    itemId: item.id,
                    kind: "multiple_choice",
                    title: item.title,
                    lockAt: item.lockAt,
                    revealAt: item.revealAt,
                    sortOrder: index,
                    status: "open",
                    currentOptionId: null,
                    correctOptionId: null,
                    correctPoints: 0,
                    incorrectPoints: 0,
                    options: item.options.map((option, sortOrder) => ({
                      ...option,
                      sortOrder,
                    })),
                    minSelections: item.minSelections ?? 1,
                    maxSelections: item.maxSelections,
                    exactPoints: 1,
                    partialPoints: 0,
                  }}
                />
              ) : (
                <SingleChoicePredictionCard
                  key={`preview-${item.id}`}
                  preview
                  poolId="preview"
                  index={index}
                  question={
                    {
                      itemId: item.id,
                      kind: "single_choice",
                      title: item.title,
                      lockAt: item.lockAt,
                      revealAt: item.revealAt,
                      sortOrder: index,
                      status: "open",
                      currentOptionId: null,
                      correctOptionId: null,
                      correctPoints: 1,
                      incorrectPoints: 0,
                      options: item.options.map((option, sortOrder) => ({
                        ...option,
                        sortOrder,
                      })),
                    } satisfies CustomQuestion
                  }
                />
              ),
            )}
          </section>
        )}
      </div>
    </PageShell>
  );
}
