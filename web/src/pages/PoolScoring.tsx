import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import {
  useCustomQuestions,
  useFootballScoring,
  useMatches,
  useMyEvents,
  usePools,
  useSetCustomResult,
  useSetMultipleChoiceResult,
  useSetNumericResult,
  useUpdateCustomScoring,
  useUpdateMultipleChoiceScoring,
  useUpdateNumericScoring,
  useUpdateFootballScoring,
} from "@/hooks/queries";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner, Label } from "@/components/ui/field";

export function PoolScoringPage() {
  const { poolId = "" } = useParams();
  const navigate = useNavigate();
  const { user, isAdmin } = useAuth();
  const pools = usePools();
  const matches = useMatches();
  const events = useMyEvents();
  const pool = pools.data?.find((item) => item.id === poolId);
  const custom = useCustomQuestions(
    pool?.event.kind === "custom" ? poolId : null,
  );
  const football = useFootballScoring(
    pool?.event.kind === "football" ? poolId : null,
  );
  const updateFootball = useUpdateFootballScoring();
  const updateCustom = useUpdateCustomScoring();
  const updateNumeric = useUpdateNumericScoring();
  const setNumericResult = useSetNumericResult();
  const updateMultiple = useUpdateMultipleChoiceScoring();
  const setMultipleResult = useSetMultipleChoiceResult();
  const setResult = useSetCustomResult();
  const [values, setValues] = useState<Record<string, string>>({});
  const [error, setError] = useState("");
  useEffect(() => {
    if (football.data)
      setValues(
        Object.fromEntries(
          Object.entries(football.data).map(([k, v]) => [k, String(v)]),
        ),
      );
  }, [football.data]);
  const owner = pool?.createdBy === user?.id || isAdmin;
  const eventOwner =
    isAdmin ||
    events.data?.some((event) => event.id === pool?.eventId) === true;
  if (pools.isLoading)
    return (
      <PageShell>
        <Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button>
        <Card>
          <p className="text-ink-muted">Carregando...</p>
        </Card>
      </PageShell>
    );
  if (!pool)
    return (
      <PageShell>
        <Button variant="link" size="sm" onClick={() => navigate("/pools")}>← Voltar aos bolões</Button>
        <div className="mt-4"><ErrorBanner>Bolão não encontrado.</ErrorBanner></div>
      </PageShell>
    );
  const footballFrozen =
    pool.event.kind === "football" &&
    (matches.data ?? []).some(
      (match) => new Date(match.kickoff).getTime() <= Date.now(),
    );
  const saveFootball = async () => {
    setError("");
    try {
      await updateFootball.mutateAsync({
        poolId,
        exactScorePoints: +values.exactScorePoints,
        correctResultExactSidePoints: +values.correctResultExactSidePoints,
        correctResultPoints: +values.correctResultPoints,
        incorrectResultPoints: +values.incorrectResultPoints,
        knockoutBonusPoints: +values.knockoutBonusPoints,
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Falha ao salvar.");
    }
  };
  return (
    <PageShell>
      <Button
        variant="link"
        size="sm"
        onClick={() => navigate(`/pools/${poolId}`)}
      >
        ← Voltar ao bolão
      </Button>
      <h1 className="mt-3 text-3xl">
        {pool.event.isHistorical ? "Regras usadas nesta edição" : "Regras de pontuação"}
      </h1>
      <p className="mt-1 text-ink-muted">
        {pool.name} · {pool.event.name}
      </p>
      {pool.event.kind === "football" ? (
        <Card className="mt-6 max-w-xl">
          {footballFrozen && owner && (
            <p className="mb-4 text-sm text-ink-muted">
              A pontuação não pode mais ser alterada porque os primeiros
              palpites já foram encerrados.
            </p>
          )}
          <div className="space-y-3">
            {[
              ["exactScorePoints", "Placar exato"],
              ["correctResultExactSidePoints", "Resultado + lado exato"],
              ["correctResultPoints", "Resultado correto"],
              ["incorrectResultPoints", "Erro"],
              ["knockoutBonusPoints", "Bônus mata-mata"],
            ].map(([key, label]) => (
              <div
                key={key}
                className="flex items-center justify-between gap-4"
              >
                <Label>{label}</Label>
                {owner && !footballFrozen ? (
                  <Input
                    className="w-24"
                    type="number"
                    min="0"
                    value={values[key] ?? ""}
                    onChange={(e) =>
                      setValues({ ...values, [key]: e.target.value })
                    }
                  />
                ) : (
                  <strong>{values[key] ?? "—"} pts</strong>
                )}
              </div>
            ))}
          </div>
          {owner && !footballFrozen && (
            <Button
              className="mt-5"
              onClick={saveFootball}
              disabled={updateFootball.isPending}
            >
              Salvar pontuação
            </Button>
          )}
        </Card>
      ) : (
        <>
          <Card className="mt-6">
            <p className="text-sm text-ink-muted">
              {pool.event.isHistorical
                ? "Consulta das regras persistidas nesta edição encerrada."
                : "Cada categoria usa a pontuação deste bolão."}
            </p>
            <div className="mt-4 space-y-3">
              {custom.data?.map((q) =>
                q.kind === "numeric" ? (
                  <NumericScoringRow
                    key={q.itemId}
                    question={q}
                    owner={owner && q.status === "open"}
                    save={(
                      exactPoints,
                      tolerance,
                      withinTolerancePoints,
                      incorrectPoints,
                    ) =>
                      updateNumeric.mutateAsync({
                        poolId,
                        itemId: q.itemId,
                        exactPoints,
                        tolerance,
                        withinTolerancePoints,
                        incorrectPoints,
                      })
                    }
                  />
                ) : q.kind === "multiple_choice" ? (
                  <MultipleScoringRow
                    key={q.itemId}
                    question={q}
                    owner={owner && q.status === "open"}
                    save={(exactPoints, partialPoints, incorrectPoints) =>
                      updateMultiple.mutateAsync({
                        poolId,
                        itemId: q.itemId,
                        exactPoints,
                        partialPoints,
                        incorrectPoints,
                      })
                    }
                  />
                ) : (
                  <CustomScoringRow
                    key={q.itemId}
                    question={q}
                    owner={owner && q.status === "open"}
                    save={(correctPoints, incorrectPoints) =>
                      updateCustom.mutateAsync({
                        poolId,
                        itemId: q.itemId,
                        correctPoints,
                        incorrectPoints,
                      })
                    }
                  />
                ),
              )}
            </div>
          </Card>
          {eventOwner && (
            <Card className="mt-5">
              <h2 className="text-xl">Resultados oficiais do evento</h2>
              <p className="mt-1 text-sm text-ink-muted">
                Esta ação é global para o Event.
              </p>
              <div className="mt-4 space-y-3">
                {custom.data?.map((q) =>
                  q.kind === "numeric" ? (
                    <NumericResultRow
                      key={q.itemId}
                      question={q}
                      save={(value) =>
                        setNumericResult.mutateAsync({
                          poolId,
                          itemId: q.itemId,
                          value,
                        })
                      }
                    />
                  ) : q.kind === "multiple_choice" ? (
                    <MultipleResultRow
                      key={q.itemId}
                      question={q}
                      save={(optionIds) =>
                        setMultipleResult.mutateAsync({
                          poolId,
                          itemId: q.itemId,
                          optionIds,
                        })
                      }
                    />
                  ) : (
                    <CustomResultRow
                      key={q.itemId}
                      question={q}
                      save={(optionId) =>
                        setResult.mutateAsync({
                          poolId,
                          itemId: q.itemId,
                          optionId,
                        })
                      }
                    />
                  ),
                )}
              </div>
            </Card>
          )}
        </>
      )}
      {error && (
        <div className="mt-4">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}
    </PageShell>
  );
}
function NumericScoringRow({
  question,
  owner,
  save,
}: {
  question: import("@/types").CustomQuestion;
  owner: boolean;
  save: (
    exact: number,
    tolerance: string,
    within: number,
    incorrect: number,
  ) => Promise<unknown>;
}) {
  const [exact, setExact] = useState(String(question.exactPoints ?? 1));
  const [tolerance, setTolerance] = useState(question.tolerance ?? "0");
  const [within, setWithin] = useState(
    String(question.withinTolerancePoints ?? 0),
  );
  const [incorrect, setIncorrect] = useState(String(question.incorrectPoints));
  return (
    <div className="border-b border-mint/15 pb-3">
      <strong>{question.title}</strong>
      <div className="mt-2 flex flex-wrap gap-2">
        {owner ? (
          <>
            <label>
              Exato
              <Input
                aria-label={`Exato: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={exact}
                onChange={(e) => setExact(e.target.value)}
              />
            </label>
            <label>
              Tolerância{question.unitLabel ? ` (${question.unitLabel})` : ""}
              <Input
                aria-label={`Tolerância: ${question.title}`}
                className="w-20"
                inputMode="decimal"
                value={tolerance}
                onChange={(e) => setTolerance(e.target.value)}
              />
            </label>
            <label>
              Dentro
              <Input
                aria-label={`Dentro da tolerância: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={within}
                onChange={(e) => setWithin(e.target.value)}
              />
            </label>
            <label>
              Fora
              <Input
                aria-label={`Fora: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={incorrect}
                onChange={(e) => setIncorrect(e.target.value)}
              />
            </label>
            <Button
              size="sm"
              variant="outline"
              onClick={() => save(+exact, tolerance, +within, +incorrect)}
            >
              Salvar
            </Button>
          </>
        ) : (
          <span className="text-sm text-ink-muted">
            Exato {exact} · tolerância {tolerance} · dentro {within} · fora{" "}
            {incorrect}
          </span>
        )}
      </div>
    </div>
  );
}
function NumericResultRow({
  question,
  save,
}: {
  question: import("@/types").CustomQuestion;
  save: (value: string) => Promise<unknown>;
}) {
  const [value, setValue] = useState(question.resultValue ?? "");
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3">
      <Label>{question.title}</Label>
      <div className="flex gap-2">
        <Input
          aria-label={`Resultado oficial: ${question.title}`}
          inputMode="decimal"
          value={value}
          onChange={(e) => setValue(e.target.value)}
        />
        <Button size="sm" disabled={!value} onClick={() => save(value)}>
          Salvar resultado
        </Button>
      </div>
    </div>
  );
}
function MultipleScoringRow({
  question,
  owner,
  save,
}: {
  question: import("@/types").CustomQuestion;
  owner: boolean;
  save: (exact: number, partial: number, incorrect: number) => Promise<unknown>;
}) {
  const [exact, setExact] = useState(String(question.exactPoints ?? 1));
  const [partial, setPartial] = useState(String(question.partialPoints ?? 0));
  const [incorrect, setIncorrect] = useState(String(question.incorrectPoints));
  return (
    <div className="border-b border-mint/15 pb-3">
      <strong>{question.title}</strong>
      <div className="mt-2 flex flex-wrap gap-2">
        {owner ? (
          <>
            <label>
              Acerto exato
              <Input
                aria-label={`Exato: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={exact}
                onChange={(e) => setExact(e.target.value)}
              />
            </label>
            <label>
              Acerto parcial
              <Input
                aria-label={`Parcial: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={partial}
                onChange={(e) => setPartial(e.target.value)}
              />
            </label>
            <label>
              Incorreto
              <Input
                aria-label={`Incorreto: ${question.title}`}
                className="w-16"
                type="number"
                min="0"
                value={incorrect}
                onChange={(e) => setIncorrect(e.target.value)}
              />
            </label>
            <Button
              size="sm"
              variant="outline"
              onClick={() => save(+exact, +partial, +incorrect)}
            >
              Salvar
            </Button>
          </>
        ) : (
          <span className="text-sm text-ink-muted">
            Exato {exact} · parcial {partial} · incorreto {incorrect}
          </span>
        )}
      </div>
    </div>
  );
}
function MultipleResultRow({
  question,
  save,
}: {
  question: import("@/types").CustomQuestion;
  save: (optionIds: string[]) => Promise<unknown>;
}) {
  const [selected, setSelected] = useState<string[]>(
    question.correctOptionIds ?? [],
  );
  const min = question.minSelections ?? 1;
  const max = question.maxSelections ?? question.options.length;
  return (
    <div className="border-b border-mint/15 pb-3">
      <Label>{question.title}</Label>
      <div className="mt-2 space-y-1">
        {question.options.map((option) => (
          <label key={option.id} className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={selected.includes(option.id)}
              disabled={!selected.includes(option.id) && selected.length >= max}
              onChange={() =>
                setSelected((old) =>
                  old.includes(option.id)
                    ? old.filter((id) => id !== option.id)
                    : [...old, option.id],
                )
              }
            />
            {option.label}
          </label>
        ))}
      </div>
      <Button
        className="mt-2"
        size="sm"
        disabled={selected.length < min || selected.length > max}
        onClick={() => save(selected)}
      >
        Salvar resultado
      </Button>
    </div>
  );
}
function CustomScoringRow({
  question,
  owner,
  save,
}: {
  question: import("@/types").CustomQuestion;
  owner: boolean;
  save: (correct: number, incorrect: number) => Promise<unknown>;
}) {
  const [correct, setCorrect] = useState(String(question.correctPoints));
  const [incorrect, setIncorrect] = useState(String(question.incorrectPoints));
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3 last:border-0">
      <span className="font-medium">{question.title}</span>
      {owner ? (
        <div className="flex items-center gap-2">
          <Input
            aria-label={`Pontos por acerto: ${question.title}`}
            className="w-16"
            type="number"
            min="0"
            value={correct}
            onChange={(e) => setCorrect(e.target.value)}
          />
          <Input
            aria-label={`Pontos por erro: ${question.title}`}
            className="w-16"
            type="number"
            min="0"
            value={incorrect}
            onChange={(e) => setIncorrect(e.target.value)}
          />
          <Button
            size="sm"
            variant="outline"
            onClick={() => save(+correct, +incorrect)}
          >
            Salvar
          </Button>
        </div>
      ) : (
        <span className="text-sm text-ink-muted">
          {question.status !== "open"
            ? "Palpites encerrados; pontuação somente leitura."
            : `${question.correctPoints} pts por acerto`}
        </span>
      )}
    </div>
  );
}
function CustomResultRow({
  question,
  save,
}: {
  question: import("@/types").CustomQuestion;
  save: (optionId: string) => Promise<unknown>;
}) {
  const [optionId, setOptionId] = useState(question.correctOptionId ?? "");
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-mint/15 pb-3 last:border-0">
      <Label>{question.title}</Label>
      <div className="flex gap-2">
        <select
          aria-label={`Vencedor: ${question.title}`}
          className="rounded-lg border border-mint/25 bg-card px-2 py-1"
          value={optionId}
          onChange={(e) => setOptionId(e.target.value)}
        >
          <option value="">Selecione</option>
          {question.options.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </select>
        <Button size="sm" disabled={!optionId} onClick={() => save(optionId)}>
          Salvar resultado
        </Button>
      </div>
    </div>
  );
}
