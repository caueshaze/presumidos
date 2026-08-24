import { fireEvent, render, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router-dom";
import { expect, it, vi } from "vitest";
import { DashboardPage } from "./Dashboard";

vi.mock("@/hooks/useAuth", () => ({
  useAuth: () => ({ user: { id: "me", username: "Cauê" }, isAdmin: false }),
}));

const mutation = { mutateAsync: vi.fn(), isPending: false };
vi.mock("@/hooks/queries", () => ({
  useDashboardPools: () => ({
    isLoading: false,
    isError: false,
    data: [
      {
        pool: { id: "active", name: "Bolão atual", inviteCode: "AB12CD", memberCount: 3, createdBy: "me", event: { id: "event-a", name: "Evento atual", slug: "atual", kind: "custom", status: "active", endsAt: "2099-01-01T00:00:00Z", isHistorical: false }, eventId: "event-a", description: "", visibleRules: "", joinClosedAt: null },
        answeredCount: 2,
        itemCount: 4,
      },
      {
        pool: { id: "history", name: "Copa da família", inviteCode: "EF34GH", memberCount: 6, createdBy: "me", event: { id: "event-b", name: "Copa do Mundo FIFA 2026", slug: "copa", kind: "football", status: "finished", endsAt: "2026-07-19T19:00:00Z", isHistorical: true }, eventId: "event-b", description: "", visibleRules: "", joinClosedAt: null },
        answeredCount: 4,
        itemCount: 4,
      },
    ],
  }),
  useMyEvents: () => ({ data: [], isLoading: false, refetch: vi.fn() }),
  useAvailableEvents: () => ({ data: [], isLoading: false, refetch: vi.fn() }),
  useCreatePool: () => mutation,
  useJoinPool: () => mutation,
  useDeletePool: () => mutation,
  usePublicSettings: () => ({ data: null }),
}));

it("focuses the dashboard on the greeting, actions, and active pools", () => {
  render(<BrowserRouter><DashboardPage /></BrowserRouter>);
  expect(screen.getByText("Olá, Cauê")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Criar bolão" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Entrar com código" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Em andamento" })).toBeTruthy();
  expect(screen.getByText("2 de 4 palpites respondidos")).toBeTruthy();
  expect(screen.queryByRole("heading", { name: "Meus eventos" })).toBeNull();
  expect(screen.queryByRole("heading", { name: "Bolões anteriores" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Criar evento" })).toBeNull();
  expect(screen.queryByText("Copa da família")).toBeNull();
});

it("explains how to create a usable event before showing a pool form", () => {
  render(<BrowserRouter><DashboardPage /></BrowserRouter>);
  fireEvent.click(screen.getByRole("button", { name: "Criar bolão" }));
  expect(screen.getByRole("heading", { name: "Antes, você precisa de um evento" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Criar meu primeiro evento" })).toBeTruthy();
  expect(screen.queryByLabelText("Evento do bolão")).toBeNull();
  expect(screen.queryByPlaceholderText("Nome do bolão (ex.: Bolão da firma)")).toBeNull();
});
