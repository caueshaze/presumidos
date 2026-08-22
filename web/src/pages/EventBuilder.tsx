import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api } from "@/lib/api";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner } from "@/components/ui/field";
import { SingleChoicePredictionCard } from "@/components/SingleChoicePredictionCard";
import { NumericPredictionCard } from "@/components/NumericPredictionCard";
import { MultipleChoicePredictionCard } from "@/components/MultipleChoicePredictionCard";
import type { CustomQuestion } from "@/types";

type Option = { id: string; label: string };
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
    startsAt: string | null;
    endsAt: string | null;
  };
  items: Item[];
};

export function EventBuilderPage() {
  const { eventId } = useParams();
  const navigate = useNavigate();
  const [draft, setDraft] = useState<Draft | null>(null);
  const [name, setName] = useState("");
  const [startsAt, setStartsAt] = useState("");
  const [endsAt, setEndsAt] = useState("");
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
  const action = async (path: string, body?: unknown) => {
    if (!draft) return;
    setBusy(true);
    try {
      await api.post(path, body);
      await load(draft.event.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Operação recusada.");
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
  const editOption = async (item: Item, option: Option) => {
    const label = window.prompt("Opção", option.label);
    if (label && draft)
      await action(
        `/custom/events/${draft.event.id}/items/${item.id}/options/${option.id}/update`,
        { label },
      );
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
        <h1 className="text-3xl">Criar evento</h1>
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
        <p>Carregando evento…</p>
        {error && <ErrorBanner>{error}</ErrorBanner>}
      </PageShell>
    );
  const editable = draft.event.status === "draft";
  return (
    <PageShell>
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
          <div className="flex gap-2">
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
          <Button
            onClick={() => navigate(`/dashboard?eventId=${draft.event.id}`)}
          >
            Criar bolão
          </Button>
        )}
      </div>
      {error && (
        <div className="mt-3">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}
      <Card className="mt-5">
        <div className="flex flex-col gap-3">
          <label>
            Nome do evento
            <Input
              disabled={!editable}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label>
            Data inicial
            <Input
              disabled={!editable}
              type="datetime-local"
              value={startsAt}
              onChange={(e) => setStartsAt(e.target.value)}
            />
          </label>
          <label>
            Data final
            <Input
              disabled={!editable}
              type="datetime-local"
              value={endsAt}
              onChange={(e) => setEndsAt(e.target.value)}
            />
          </label>
          {editable && (
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
                <ol className="mt-3 list-decimal pl-5">
                  {item.options.map((o, optionIndex) => (
                    <li key={o.id} className="flex justify-between gap-2">
                      <span>{o.label}</span>
                      {editable && (
                        <span className="flex gap-1">
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={() => void editOption(item, o)}
                          >
                            Editar
                          </Button>
                          <Button
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
                          </Button>
                          <Button
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
                          </Button>
                          <Button
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
                          </Button>
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
                  <div className="mt-3 flex gap-2">
                    <select
                      aria-label={`Resultado oficial: ${item.title}`}
                      value={results[item.id] ?? item.correctOptionId ?? ""}
                      onChange={(e) =>
                        setResults((v) => ({ ...v, [item.id]: e.target.value }))
                      }
                    >
                      <option value="">Selecione</option>
                      {item.options.map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.label}
                        </option>
                      ))}
                    </select>
                    <Button
                      size="sm"
                      disabled={!(results[item.id] ?? item.correctOptionId)}
                      onClick={() =>
                        action(`/admin/custom/questions/${item.id}/result`, {
                          optionId: results[item.id] ?? item.correctOptionId,
                        })
                      }
                    >
                      Salvar resultado
                    </Button>
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
