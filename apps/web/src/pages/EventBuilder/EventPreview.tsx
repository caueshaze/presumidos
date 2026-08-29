import { SingleChoicePredictionCard } from "@/components/SingleChoicePredictionCard";
import { NumericPredictionCard } from "@/components/NumericPredictionCard";
import { MultipleChoicePredictionCard } from "@/components/MultipleChoicePredictionCard";
import type { CustomQuestion } from "@/types";
import type { Item } from "@/components/EventBuilderItems";

export function EventPreview({ items }: { items: Item[] }) {
  if (!items.length) return null;
  return (
          <section>
            <h2 className="mb-3 text-2xl">Prévia</h2>
            {items.map((item, index) =>
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
  );
}
