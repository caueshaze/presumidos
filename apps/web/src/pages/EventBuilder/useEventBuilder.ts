import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api } from "@/lib/api";
import { withAdminReauth } from "@/lib/adminReauth";
import type { EventVersionHistory, OptionLink } from "@/types";
import { useAuth } from "@/hooks/useAuth";
import { useDeleteEvent, useReauth } from "@/hooks/queries";
import { type Item, type Option } from "@/components/EventBuilderItems";
import { toIsoDateTime } from "@/components/PtBrDateTimeInput";
import { eventCreationError, type Draft } from "./types";


export function useEventBuilder() {
  const { eventId } = useParams();
  const navigate = useNavigate();
  const { isAdmin, user } = useAuth();
  const reauth = useReauth();
  const deleteEvent = useDeleteEvent();
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
  const [showAddItemForm, setShowAddItemForm] = useState(false);
  const [openAddOptionItemId, setOpenAddOptionItemId] = useState<string | null>(null);
  const [labels, setLabels] = useState<Record<string, string>>({});
  const [mediaDrafts, setMediaDrafts] = useState<Record<string, { imageUrl: string; links: OptionLink[] }>>({});
  const [openMediaOptionId, setOpenMediaOptionId] = useState<string | null>(null);
  const [editingOptionId, setEditingOptionId] = useState<string | null>(null);
  const [optionLabelDraft, setOptionLabelDraft] = useState("");
  const [editingItemId, setEditingItemId] = useState<string | null>(null);
  const [itemTitleDraft, setItemTitleDraft] = useState("");
  const [itemLockDraft, setItemLockDraft] = useState("");
  const [itemRevealDraft, setItemRevealDraft] = useState("");
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
    setError("");
    if (startsAt && endsAt && new Date(startsAt).getTime() >= new Date(endsAt).getTime()) {
      setError("A data inicial deve ser anterior à data final.");
      return;
    }
    setBusy(true);
    try {
      const event = await api.post<{ id: string }>("/custom/events", {
        name,
        startsAt: startsAt ? new Date(startsAt).toISOString() : null,
        endsAt: endsAt ? new Date(endsAt).toISOString() : null,
      });
      navigate(`/events/${event.id}`);
    } catch (e) {
      setError(eventCreationError(e));
    } finally {
      setBusy(false);
    }
  };
  const addItem = async (e: FormEvent) => {
    e.preventDefault();
    if (!draft) return;
    const revealAt = draft.event.endsAt || lockAt;
    if (!title.trim() || !lockAt || !revealAt) {
      setError("Preencha a pergunta e a data de fechamento dos palpites.");
      return;
    }
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
              lockAt: toIsoDateTime(lockAt),
              revealAt: toIsoDateTime(revealAt),
              decimalPlaces: Number(decimalPlaces),
              unitLabel: unitLabel || null,
              minValue: minValue || null,
              maxValue: maxValue || null,
            }
          : itemKind === "multiple_choice"
            ? {
                title,
                lockAt: toIsoDateTime(lockAt),
                revealAt: toIsoDateTime(revealAt),
                minSelections: Number(minSelections),
                maxSelections: maxSelections ? Number(maxSelections) : null,
              }
            : { title, lockAt: toIsoDateTime(lockAt), revealAt: toIsoDateTime(revealAt) };
      await api.post(`/custom/events/${draft.event.id}/items${path}`, body);
      setTitle("");
      setUnitLabel("");
      setMinValue("");
      setMaxValue("");
      setMinSelections("1");
      setMaxSelections("");
      setShowAddItemForm(false);
      await load(draft.event.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Falha ao adicionar pergunta.");
    } finally {
      setBusy(false);
    }
  };
  const addOption = async (item: Item) => {
    const label = labels[item.id]?.trim();
    if (!label || !draft) return;
    const added = await action(
      `/custom/events/${draft.event.id}/items/${item.id}/options`,
      { label },
    );
    if (added) {
      setLabels((current) => ({ ...current, [item.id]: "" }));
      setOpenAddOptionItemId(null);
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
  const startItemEdit = (item: Item) => {
    setError("");
    setEditingItemId(item.id);
    setItemTitleDraft(item.title);
    setItemLockDraft(item.lockAt);
    setItemRevealDraft(item.revealAt);
  };
  const cancelItemEdit = () => {
    setEditingItemId(null);
    setItemTitleDraft("");
    setItemLockDraft("");
    setItemRevealDraft("");
  };
  const saveItemEdit = async (item: Item) => {
    const title = itemTitleDraft.trim();
    const lockAt = itemLockDraft.trim();
    const revealAt = itemRevealDraft.trim();
    if (!title || !lockAt || !revealAt) {
      setError("Preencha a pergunta, a data de fechamento e a data de revelação.");
      return;
    }
    if (!draft) return;
    const saved = await action(`/custom/events/${draft.event.id}/items/${item.id}/update`, {
      title,
      lockAt: toIsoDateTime(lockAt),
      revealAt: toIsoDateTime(revealAt),
    });
    if (saved) cancelItemEdit();
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
  const deleteOwnedEvent = async () => {
    if (!draft) return;
    const hasPools = draft.versions.some((version) => version.poolCount > 0);
    const willArchive = draft.event.status !== "draft" || hasPools;
    const confirmation = willArchive
      ? `Arquivar o evento "${draft.event.name}"? Ele sairá dos catálogos, mas os bolões existentes continuarão preservados.`
      : `Excluir definitivamente o rascunho "${draft.event.name}"? Esta ação não pode ser desfeita.`;
    if (!window.confirm(confirmation))
      return;
    setBusy(true);
    try {
      await deleteEvent.mutateAsync(draft.event.id);
      navigate("/events");
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Não foi possível remover o evento.",
      );
    } finally {
      setBusy(false);
    }
  };
  const restoreVersion = async (version: EventVersionHistory) => {
    if (!draft || !window.confirm(`Restaurar a V${version.versionNumber}? Isso substituirá a revisão de trabalho atual, sem alterar Pools existentes.`)) return;
    setEditingItemId(null);
    setEditingOptionId(null);
    setOpenMediaOptionId(null);
    setBusy(true);
    setError("");
    try {
      await withAdminReauth(
        () => api.post(`/admin/events/${draft.event.id}/versions/${version.id}/restore`),
        (password) => reauth.mutateAsync(password),
      );
      await load(draft.event.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Não foi possível restaurar a versão.");
    } finally {
      setBusy(false);
    }
  };
  return { eventId, navigate, isAdmin, user, reauth, deleteEvent, draft, setDraft, name, setName, startsAt, setStartsAt, endsAt, setEndsAt, description, setDescription, coverUrl, setCoverUrl, externalUrl, setExternalUrl, title, setTitle, itemKind, setItemKind, decimalPlaces, setDecimalPlaces, unitLabel, setUnitLabel, minValue, setMinValue, maxValue, setMaxValue, minSelections, setMinSelections, maxSelections, setMaxSelections, lockAt, setLockAt, showAddItemForm, setShowAddItemForm, openAddOptionItemId, setOpenAddOptionItemId, labels, setLabels, mediaDrafts, setMediaDrafts, openMediaOptionId, setOpenMediaOptionId, editingOptionId, setEditingOptionId, optionLabelDraft, setOptionLabelDraft, editingItemId, setEditingItemId, itemTitleDraft, setItemTitleDraft, itemLockDraft, setItemLockDraft, itemRevealDraft, setItemRevealDraft, results, setResults, multipleResults, setMultipleResults, error, setError, busy, setBusy, load, create, addItem, addOption, action, saveMetadata, startItemEdit, cancelItemEdit, saveItemEdit, startOptionEdit, cancelOptionEdit, saveOptionLabel, saveOptionMedia, downloadEventFile, deleteOwnedEvent, restoreVersion };
}
