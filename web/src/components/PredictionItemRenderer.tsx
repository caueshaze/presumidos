import type { ComponentProps } from "react";
import type { CustomQuestion } from "@/types";
import { MatchCard } from "./MatchCard";
import { SingleChoicePredictionCard } from "./SingleChoicePredictionCard";

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

type PredictionItemRendererProps =
  | { item: FootballPredictionItem }
  | { item: SingleChoicePredictionItem };

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
  }
}
