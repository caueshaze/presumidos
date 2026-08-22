import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("./MatchCard", () => ({
  MatchCard: () => <div data-testid="football-card">football</div>,
}));
vi.mock("./SingleChoicePredictionCard", () => ({
  SingleChoicePredictionCard: ({ question }: { question: { title: string } }) => (
    <div data-testid="single-choice-card">{question.title}</div>
  ),
}));

import { PredictionItemRenderer } from "./PredictionItemRenderer";

describe("PredictionItemRenderer", () => {
  it("dispatches football_match to the football card", () => {
    render(
      <PredictionItemRenderer
        item={{
          kind: "football_match",
          match: {
            game: {} as never,
            locked: false,
            isAdmin: false,
            index: 0,
          },
        }}
      />,
    );
    expect(screen.getByTestId("football-card")).toBeTruthy();
  });

  it("dispatches single_choice to the generic choice card", () => {
    render(
      <PredictionItemRenderer
        item={{
          kind: "single_choice",
          poolId: "pool-a",
          index: 0,
          question: {
            itemId: "item-a",
            kind: "single_choice",
            title: "Pergunta genérica",
            lockAt: "2099-01-01T00:00:00Z",
            revealAt: "2099-01-02T00:00:00Z",
            sortOrder: 0,
            status: "open",
            currentOptionId: null,
            correctOptionId: null,
            correctPoints: 1,
            incorrectPoints: 0,
            options: [],
          },
        }}
      />,
    );
    expect(screen.getByTestId("single-choice-card").textContent).toContain("Pergunta genérica");
  });
});
