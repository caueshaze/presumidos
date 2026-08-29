import { PageShell } from "@/components/PageShell";
import { PredictionItemRenderer } from "@/components/PredictionItemRenderer";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";

export function CustomPredictionsView({ context }: { context: Record<string, any> }) {
  const { navigate, poolId, currentPool, customQuestions } = context;
    const questions = customQuestions.data ?? [];
    const answered = questions.filter((question: any) => question.kind === "numeric" ? question.currentValue != null : question.kind === "multiple_choice" ? (question.currentOptionIds?.length ?? 0) > 0 : question.currentOptionId != null).length;
    return (
      <PageShell>
        <Button variant="link" size="sm" onClick={() => navigate(poolId ? `/pools/${poolId}` : "/pools")}>
          ← Voltar ao bolão
        </Button>
        <h1 className="mt-3 text-3xl">Palpites</h1>
        <p className="mt-1 text-ink-muted">
          {currentPool.name} · {currentPool.event.name}
        </p>
        {currentPool.event.isHistorical && (
          <p className="mt-2 text-sm font-semibold text-mint-dark">
            Edição encerrada — consulte seus palpites e os resultados oficiais.
          </p>
        )}
        {!customQuestions.isLoading && (
          <p className="mt-3 text-sm font-semibold text-mint-dark">
            {answered} de {questions.length} categorias respondidas
          </p>
        )}
        <div className="mt-6 space-y-4">
          {customQuestions.isLoading ? (
            <Card><p className="text-ink-muted">Carregando categorias...</p></Card>
          ) : customQuestions.isError ? (
            <ErrorBanner>
              Erro ao carregar categorias: {(customQuestions.error as Error).message}
            </ErrorBanner>
          ) : (
            questions.map((question: any, index: number) => question.kind === "numeric" ? (
              <PredictionItemRenderer key={question.itemId} item={{ kind: "numeric", question, poolId: poolId!, index }} />
            ) : question.kind === "multiple_choice" ? (
              <PredictionItemRenderer key={question.itemId} item={{ kind: "multiple_choice", question, poolId: poolId!, index }} />
            ) : (
              <PredictionItemRenderer key={question.itemId} item={{ kind: "single_choice", question, poolId: poolId!, index }} />
            ))
          )}
        </div>
      </PageShell>
    );
}
