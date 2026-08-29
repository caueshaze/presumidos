import { useEffect, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import {
  useCustomQuestions,
  useFootballScoring,
  useMatches,
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
import { NumericResultRow, NumericScoringRow } from "./pool-scoring/NumericRows";
import { MultipleResultRow, MultipleScoringRow } from "./pool-scoring/MultipleRows";
import { CustomResultRow, CustomScoringRow } from "./pool-scoring/CustomRows";

export function PoolScoringPage() {
  const { poolId = "" } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { user, isAdmin } = useAuth();
  const pools = usePools();
  const matches = useMatches();
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
  const [customSection, setCustomSection] = useState<"scoring" | "results">(
    searchParams.get("section") === "results" ? "results" : "scoring",
  );
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
      {pool.event.kind === "custom" && owner && (
        <div className="mt-5 flex flex-wrap gap-2" role="tablist" aria-label="Administração do evento">
          <Button variant={customSection === "scoring" ? "primary" : "outline"} size="sm" role="tab" aria-selected={customSection === "scoring"} onClick={() => setCustomSection("scoring")}>Regras de pontuação</Button>
          <Button variant={customSection === "results" ? "primary" : "outline"} size="sm" role="tab" aria-selected={customSection === "results"} onClick={() => setCustomSection("results")}>Resultados oficiais</Button>
        </div>
      )}
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
          {(!owner || customSection === "scoring") && <Card className="mt-6">
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
          </Card>}
          {owner && customSection === "results" && (
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
