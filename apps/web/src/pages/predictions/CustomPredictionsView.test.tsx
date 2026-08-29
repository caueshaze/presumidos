import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CustomPredictionsView } from "./CustomPredictionsView";

vi.mock("@/components/PredictionItemRenderer", () => ({
  PredictionItemRenderer: ({ item }: { item: { question: { title: string } } }) => <article>{item.question.title}</article>,
}));

const questions = [
  { itemId: "answered", kind: "single_choice", title: "Já respondida", currentOptionId: "option-1" },
  { itemId: "pending", kind: "single_choice", title: "Próxima pendência", currentOptionId: null },
];

it("shows progress and scrolls to the first unanswered category", () => {
  const scrollIntoView = vi.fn();
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: scrollIntoView });
  render(<CustomPredictionsView context={{ navigate: vi.fn(), poolId: "pool-1", currentPool: { name: "Meu bolão", event: { name: "Meu evento" } }, customQuestions: { data: questions, isLoading: false, isError: false } }} />);

  expect(screen.getByText("1 de 2 categorias respondidas")).toBeTruthy();
  expect(screen.getByRole("progressbar", { name: "Progresso dos palpites" }).getAttribute("aria-valuenow")).toBe("1");
  fireEvent.click(screen.getByRole("button", { name: "Próximo palpite" }));
  expect(scrollIntoView).toHaveBeenCalledWith({ behavior: "smooth", block: "start" });
});

it("hides the shortcut when every category has a prediction", () => {
  render(<CustomPredictionsView context={{ navigate: vi.fn(), poolId: "pool-1", currentPool: { name: "Meu bolão", event: { name: "Meu evento" } }, customQuestions: { data: [{ ...questions[0] }], isLoading: false, isError: false } }} />);
  expect(screen.queryByRole("button", { name: "Próximo palpite" })).toBeNull();
});
