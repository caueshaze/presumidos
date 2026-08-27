import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { expect, it, vi } from "vitest";
import { PoolOverviewPage } from "./PoolOverview";

const pool = {
  id: "pool-1",
  name: "Bolão dos cria",
  eventId: "event-1",
  event: {
    id: "event-1",
    name: "Evento atual",
    slug: "evento-atual",
    kind: "custom" as const,
    status: "active" as const,
    endsAt: "2099-09-27T19:00:00Z",
    isHistorical: false,
  },
  inviteCode: "3A44F6",
  memberCount: 4,
  createdBy: "user-1",
  description: "",
  visibleRules: "",
  joinClosedAt: null,
};

vi.mock("@/hooks/useAuth", () => ({
  useAuth: () => ({ user: { id: "user-1" } }),
}));

vi.mock("@/hooks/queries", () => ({
  usePools: () => ({ isLoading: false, data: [pool] }),
  useDashboardPools: () => ({ isLoading: false, data: [{ pool, answeredCount: 2, itemCount: 5 }] }),
  useLeaderboard: () => ({ data: [] }),
  useEventShowcase: () => ({ data: null }),
  useLeavePool: () => ({ mutateAsync: vi.fn(), isPending: false, error: null }),
  useDeletePool: () => ({ mutateAsync: vi.fn(), isPending: false, error: null }),
  useCreatePoolReport: () => ({ mutateAsync: vi.fn(), isPending: false, error: null, reset: vi.fn() }),
}));

function renderOverview() {
  return render(
    <MemoryRouter initialEntries={["/pools/pool-1"]}>
      <Routes>
        <Route path="/pools/:poolId" element={<PoolOverviewPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

it("opens one sharing modal with the invite link and code", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.assign(navigator, { clipboard: { writeText } });
  renderOverview();

  fireEvent.click(screen.getByRole("button", { name: "Opções" }));
  expect(screen.getByRole("menuitem", { name: /Excluir bolão/ })).toBeTruthy();
  expect(screen.queryByRole("menuitem", { name: /Sair do bolão/ })).toBeNull();
  fireEvent.pointerDown(document.body);
  expect(screen.queryByRole("menu")).toBeNull();

  expect(screen.getAllByRole("button", { name: "Compartilhar" })).toHaveLength(1);
  expect(screen.queryByRole("button", { name: "Copiar link" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Copiar código" })).toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "Compartilhar" }));

  expect(screen.getByRole("dialog", { name: "Compartilhar bolão" })).toBeTruthy();
  expect(screen.queryByText(/\/pools\/join\/3A44F6/)).toBeNull();
  expect(screen.getByText("3A44F6")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Copiar código" }));
  await waitFor(() => expect(writeText).toHaveBeenCalledWith("3A44F6"));

  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("shows leaving to participants instead of pool deletion", () => {
  pool.createdBy = "owner-2";
  renderOverview();
  fireEvent.click(screen.getByRole("button", { name: "Opções" }));
  expect(screen.getByRole("menuitem", { name: /Sair do bolão/ })).toBeTruthy();
  expect(screen.queryByRole("menuitem", { name: /Excluir bolão/ })).toBeNull();
  pool.createdBy = "user-1";
});
