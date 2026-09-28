import { render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CustomScoringRow } from "./CustomRows";

it("shows configured scores after predictions close", () => {
  render(
    <CustomScoringRow
      question={{ itemId: "item-1", kind: "single_choice", title: "Campeão", lockAt: "2026-01-01T00:00:00Z", revealAt: "2026-01-02T00:00:00Z", sortOrder: 0, status: "locked", currentOptionId: null, correctOptionId: null, correctPoints: 5, incorrectPoints: 1, options: [] }}
      owner={false}
      save={vi.fn()}
    />,
  );

  expect(screen.getByText("Acerto 5 pts · Erro 1 pts")).toBeTruthy();
  expect(screen.queryByText(/pontuação somente leitura/i)).toBeNull();
});
