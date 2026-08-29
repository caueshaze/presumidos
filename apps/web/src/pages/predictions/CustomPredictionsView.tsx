import { PageShell } from "@/components/PageShell";
import { PredictionItemRenderer } from "@/components/PredictionItemRenderer";
import { ArrowDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorBanner } from "@/components/ui/field";

export function CustomPredictionsView({ context }: { context: Record<string, any> }) {
  const { navigate, poolId, currentPool, customQuestions } = context;
    const questions = customQuestions.data ?? [];
    const isAnswered = (question: any) => question.kind === "numeric" ? question.currentValue != null : question.kind === "multiple_choice" ? (question.currentOptionIds?.length ?? 0) > 0 : question.currentOptionId != null;
    const answered = questions.filter(isAnswered).length;
    const nextUnanswered = questions.find((question: any) => !isAnswered(question));
    const progress = questions.length ? Math.round((answered / questions.length) * 100) : 0;
    const goToNextUnanswered = () => {
      document.getElementById(`prediction-item-${nextUnanswered?.itemId}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
    };
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
          <div className="mt-3 max-w-md">
            <p className="text-sm font-semibold text-mint-dark">{answered} de {questions.length} categorias respondidas</p>
            <div className="mt-2 h-2 overflow-hidden rounded-pill bg-mint/15" role="progressbar" aria-label="Progresso dos palpites" aria-valuemin={0} aria-valuemax={questions.length} aria-valuenow={answered}>
              <div className="h-full rounded-pill bg-mint-dark transition-[width]" style={{ width: `${progress}%` }} />
            </div>
            {nextUnanswered && <Button variant="outline" size="sm" className="mt-3" onClick={goToNextUnanswered}>Próximo palpite <ArrowDown className="h-4 w-4" /></Button>}
          </div>
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
              <div id={`prediction-item-${question.itemId}`} key={question.itemId}><PredictionItemRenderer item={{ kind: "numeric", question, poolId: poolId!, index }} /></div>
            ) : question.kind === "multiple_choice" ? (
              <div id={`prediction-item-${question.itemId}`} key={question.itemId}><PredictionItemRenderer item={{ kind: "multiple_choice", question, poolId: poolId!, index }} /></div>
            ) : (
              <div id={`prediction-item-${question.itemId}`} key={question.itemId}><PredictionItemRenderer item={{ kind: "single_choice", question, poolId: poolId!, index }} /></div>
            ))
          )}
        </div>
      </PageShell>
    );
}
