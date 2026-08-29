import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { beforeEach, expect, it, vi } from "vitest";
import type { PublicPoolInvitePreview } from "@/types";
import { PoolInvitePage } from "./PoolInvite";

const mocks = vi.hoisted(() => ({
  auth: { user: null as { id: string } | null },
  preview: {
    isLoading: false,
    isError: false,
    refetch: vi.fn(),
    data: {
      poolName: "Bolão dos cria",
      eventName: "Evento congelado",
      eventDescription: "Uma descrição pública.",
      coverAssetUrl: "/media/assets/cover/cover",
      coverUrl: "https://cdn.example/cover.webp",
      creatorDisplayName: "Ana",
      memberCount: 7,
      lockDeadline: "2099-09-27T19:00:00Z",
      joinStatus: "joinable",
      poolId: null,
      } as PublicPoolInvitePreview,
  },
  join: { mutateAsync: vi.fn(), isPending: false, isError: false, error: null },
}));

const defaultData = { ...mocks.preview.data };

vi.mock("@/hooks/useAuth", () => ({ useAuth: () => mocks.auth }));
vi.mock("@/hooks/queries", () => ({
  usePublicPoolInvitePreview: () => mocks.preview,
  useJoinPool: () => mocks.join,
}));

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="location">{location.pathname}</span>;
}

function renderInvite() {
  return render(
    <MemoryRouter initialEntries={["/pools/join/ABC123"]}>
      <Routes>
        <Route path="/pools/join/:token" element={<PoolInvitePage />} />
        <Route path="*" element={<LocationProbe />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  mocks.auth.user = null;
  mocks.preview.data = { ...defaultData };
  mocks.join.mutateAsync.mockReset();
});

it("shows the public pool preview and falls back from the internal cover", async () => {
  renderInvite();

  expect(screen.getByRole("heading", { name: "Bolão dos cria" })).toBeTruthy();
  expect(screen.getByText("Evento congelado")).toBeTruthy();
  expect(screen.getByText("Ana convidou você para participar.")).toBeTruthy();
  expect(screen.getByText("7 participante(s)")).toBeTruthy();
  expect(document.title).toBe("Bolão dos cria — Presumidos");
  expect(screen.queryByText(/ranking/i)).toBeNull();

  const image = screen.getByRole("img");
  expect(image.getAttribute("src")).toBe("/media/assets/cover/cover");
  fireEvent.error(image);
  expect(image.getAttribute("src")).toBe("https://cdn.example/cover.webp");

  Object.assign(navigator, { clipboard: { writeText: vi.fn().mockResolvedValue(undefined) } });
  fireEvent.click(screen.getByRole("button", { name: "Copiar link" }));
  await waitFor(() => expect(screen.getByText("Link copiado!")).toBeTruthy());
});

it("joins and opens the pool for an authenticated user", async () => {
  mocks.auth.user = { id: "member-1" };
  mocks.join.mutateAsync.mockResolvedValueOnce({ id: "pool-1" });
  renderInvite();

  fireEvent.click(screen.getByRole("button", { name: /Entrar neste bolão/ }));
  await waitFor(() => expect(screen.getByTestId("location").textContent).toBe("/pools/pool-1"));
  expect(mocks.join.mutateAsync).toHaveBeenCalledWith("ABC123");
});

it("shows the existing membership action without another join CTA", () => {
  mocks.auth.user = { id: "member-1" };
  mocks.preview.data = { ...defaultData, joinStatus: "already_member", poolId: "pool-1" };
  renderInvite();

  expect(screen.getByText("Você já participa deste bolão.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: /Entrar neste bolão/ })).toBeNull();
  expect(screen.getByRole("button", { name: /Abrir bolão/ })).toBeTruthy();
});

it("renders a closed invite without an image or join CTA", () => {
  mocks.preview.data = {
    ...defaultData,
    coverAssetUrl: null,
    coverUrl: null,
    joinStatus: "closed",
  };
  renderInvite();

  expect(screen.queryByRole("img")).toBeNull();
  expect(screen.getByText("Este bolão não aceita mais participantes.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: /Entrar neste bolão/ })).toBeNull();
});

it("renders an invalid invite without public details", () => {
  mocks.preview.data = {
    ...defaultData,
    poolName: null,
    eventName: null,
    eventDescription: null,
    creatorDisplayName: null,
    memberCount: null,
    lockDeadline: null,
    coverAssetUrl: null,
    coverUrl: null,
    joinStatus: "invalid",
    poolId: null,
  };
  renderInvite();

  expect(screen.getByRole("heading", { name: "Este convite não é válido" })).toBeTruthy();
  expect(screen.queryByText("Bolão dos cria")).toBeNull();
});
