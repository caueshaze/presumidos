import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { MultipleChoicePredictionCard } from "./MultipleChoicePredictionCard";
import { setCsrfToken } from "@/lib/api";
import type { CustomQuestion } from "@/types";

const question: CustomQuestion = {
  itemId: "multiple-a",
  kind: "multiple_choice",
  title: "Quais artistas?",
  lockAt: "2099-01-01T00:00:00Z",
  revealAt: "2099-01-02T00:00:00Z",
  sortOrder: 0,
  status: "open",
  currentOptionId: null,
  correctOptionId: null,
  correctPoints: 0,
  incorrectPoints: 0,
  options: [
    { id: "a", label: "A", sortOrder: 0 },
    { id: "b", label: "B", sortOrder: 1 },
    { id: "c", label: "C", sortOrder: 2 },
  ],
  minSelections: 1,
  maxSelections: 2,
  currentOptionIds: [],
};
function renderCard(overrides: Partial<CustomQuestion> = {}) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  client.setQueryData(
    ["custom-questions", "pool-a"],
    [{ ...question, ...overrides }],
  );
  return render(
    <QueryClientProvider client={client}>
      <MultipleChoicePredictionCard
        question={{ ...question, ...overrides }}
        poolId="pool-a"
        index={0}
      />
    </QueryClientProvider>,
  );
}
afterEach(() => {
  vi.unstubAllGlobals();
  setCsrfToken(null);
});
it("submits the complete selected set and enforces max selections", async () => {
  const fetchMock = vi
    .fn()
    .mockResolvedValueOnce(new Response(JSON.stringify({ csrfToken: "csrf" })))
    .mockResolvedValue(new Response(null, { status: 204 }));
  vi.stubGlobal("fetch", fetchMock);
  renderCard();
  fireEvent.click(screen.getByRole("checkbox", { name: "A" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "C" }));
  expect(
    (screen.getByRole("checkbox", { name: "B" }) as HTMLInputElement).disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Salvar palpite" }));
  await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({
    poolId: "pool-a",
    itemId: "multiple-a",
    optionIds: ["a", "c"],
  });
});
it("rejects save below minimum", () => {
  renderCard({ minSelections: 2 });
  expect(
    (
      screen.getByRole("button", {
        name: "Salvar palpite",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
});
it("locks controls", () => {
  renderCard({ status: "locked" });
  expect(
    (screen.getByRole("checkbox", { name: "A" }) as HTMLInputElement).disabled,
  ).toBe(true);
});
