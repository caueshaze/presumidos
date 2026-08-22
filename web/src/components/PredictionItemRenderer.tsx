import type { ComponentProps } from "react";
import type { CustomQuestion } from "@/types";
import { MatchCard } from "./MatchCard";
import { SingleChoicePredictionCard } from "./SingleChoicePredictionCard";
import { NumericPredictionCard } from "./NumericPredictionCard";
import { MultipleChoicePredictionCard } from "./MultipleChoicePredictionCard";

type FootballPredictionItem = {
  kind: "football_match";
  match: ComponentProps<typeof MatchCard>;
};

type SingleChoicePredictionItem = {
  kind: "single_choice";
  question: CustomQuestion;
  poolId: string;
  index: number;
};
type NumericPredictionItem = { kind: "numeric"; question: CustomQuestion; poolId: string; index: number };
type MultipleChoicePredictionItem = { kind: "multiple_choice"; question: CustomQuestion; poolId: string; index: number };

type PredictionItemRendererProps =
  | { item: FootballPredictionItem }
  | { item: SingleChoicePredictionItem }
  | { item: NumericPredictionItem }
  | { item: MultipleChoicePredictionItem };

/** Renderiza a experiência própria de cada PredictionItem sem conhecer eventos concretos. */
export function PredictionItemRenderer({ item }: PredictionItemRendererProps) {
  switch (item.kind) {
    case "football_match":
      return <MatchCard {...item.match} />;
    case "single_choice":
      return (
        <SingleChoicePredictionCard
          question={item.question}
          poolId={item.poolId}
          index={item.index}
        />
      );
    case "numeric": return <NumericPredictionCard question={item.question} poolId={item.poolId} index={item.index} />;
    case "multiple_choice": return <MultipleChoicePredictionCard question={item.question} poolId={item.poolId} index={item.index} />;
  }
}
