import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SingleChoicePredictionCard } from "./SingleChoicePredictionCard";
import type { CustomQuestion } from "@/types";

const question: CustomQuestion = {
  itemId: "category-1",
  kind: "single_choice",
  title: "Categoria aberta",
  lockAt: "2099-01-01T00:00:00Z",
  revealAt: "2099-01-02T00:00:00Z",
  sortOrder: 0,
  status: "open",
  currentOptionId: "option-a",
  correctOptionId: null,
  correctPoints: 5,
  incorrectPoints: 0,
  options: [
    { id: "option-a", label: "Opção A", sortOrder: 0 },
    { id: "option-b", label: "Opção B", sortOrder: 1 },
  ],
};

function renderCard(overrides: Partial<CustomQuestion> = {}) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <SingleChoicePredictionCard question={{ ...question, ...overrides }} poolId="pool-a" index={0} />
    </QueryClientProvider>,
  );
}

afterEach(() => vi.unstubAllGlobals());

describe("SingleChoicePredictionCard", () => {
  it("renders options, saved selection and pool-specific score", () => {
    renderCard();
    expect(screen.getByText("1")).toBeTruthy();
    expect(screen.getByText("2")).toBeTruthy();
    expect((screen.getByRole("radio", { name: "Opção A" }) as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText("Vale 5 pontos")).toBeTruthy();
    expect(screen.getByRole("radio", { name: "Opção B" })).toBeTruthy();
  });

  it("updates an existing prediction with only permitted identifiers", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ csrfToken: "csrf" }), { status: 200 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);
    renderCard();
    fireEvent.click(screen.getByRole("radio", { name: "Opção B" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(fetchMock.mock.calls[0][0]).toBe("/api/auth/csrf");
    expect(fetchMock.mock.calls[1][0]).toBe("/api/custom/predictions");
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({
      poolId: "pool-a",
      itemId: "category-1",
      optionId: "option-b",
    });
  });

  it("is read-only after lock", () => {
    renderCard({ status: "locked" });
    expect(screen.getByRole("radio", { name: "Opção A" }).closest("fieldset")?.disabled).toBe(true);
    expect(screen.getByText("Palpites encerrados")).toBeTruthy();
  });

  it("falls back from an unavailable internal asset to the external URL", () => {
    renderCard({
      options: [{ id: "option-a", label: "Opção A", sortOrder: 0, imageAssetUrl: "/media/a/card", imageUrl: "https://example.test/a.jpg" }, ...question.options.slice(1)],
    });
    const image = document.querySelector("img") as HTMLImageElement;
    expect(image.src).toContain("/media/a/card");
    fireEvent.error(image);
    expect(image.src).toBe("https://example.test/a.jpg");
  });

  it("keeps builder preview interactions local", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
    render(<QueryClientProvider client={client}><SingleChoicePredictionCard question={question} poolId="preview" index={0} preview /></QueryClientProvider>);
    fireEvent.click(screen.getByRole("radio", { name: "Opção B" }));
    await waitFor(() => expect((screen.getByRole("radio", { name: "Opção B" }) as HTMLInputElement).checked).toBe(true));
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
