import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { AdminManifestPanel } from "./AdminManifestPanel";

const { post, upload } = vi.hoisted(() => ({ post: vi.fn(), upload: vi.fn() }));

vi.mock("@/lib/api", () => ({
  api: { post, upload },
}));

vi.mock("@/hooks/queries", () => ({
  useReauth: () => ({ mutateAsync: vi.fn() }),
}));

function renderPanel() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <AdminManifestPanel />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  post.mockReset();
  upload.mockReset();
});

it("uploads JSON and shows the backend preview before apply", async () => {
  post.mockResolvedValueOnce({
    action: "create",
    name: "Evento de teste",
    slug: "evento-de-teste",
    schemaVersion: 1,
    itemCount: 3,
    optionCount: 4,
    linkCount: 1,
    manifestFingerprint: "manifest",
    baseFingerprint: "base",
    safeChanges: [],
    blockedChanges: [],
  });
  renderPanel();
  const file = new File(["{}"], "evento.json", { type: "application/json" });
  fireEvent.change(screen.getByLabelText("Manifesto ou pacote"), { target: { files: [file] } });
  fireEvent.click(screen.getByRole("button", { name: "Validar e visualizar" }));
  await waitFor(() => expect(screen.getByText("Evento de teste")).toBeTruthy());
  expect(screen.getByText(/Nenhuma alteração é aplicada nessa etapa/)).toBeTruthy();
  expect(post).toHaveBeenCalledWith("/admin/events/import/preview", expect.objectContaining({ filename: "evento.json" }));
});

it("uploads a package and shows asset counts in the preview", async () => {
  upload.mockResolvedValueOnce({
    manifest: {
      action: "create",
      name: "Evento com imagens",
      slug: "evento-com-imagens",
      schemaVersion: 2,
      itemCount: 1,
      optionCount: 2,
      linkCount: 0,
      manifestFingerprint: "manifest",
      baseFingerprint: "base",
      safeChanges: [],
      blockedChanges: [],
    },
    assetCount: 3,
    existingAssetCount: 1,
    addedAssetCount: 2,
  });
  renderPanel();
  const file = new File([new Uint8Array([80, 75, 3, 4])], "evento-com-imagens.zip", { type: "application/zip" });
  fireEvent.change(screen.getByLabelText("Manifesto ou pacote"), { target: { files: [file] } });
  fireEvent.click(screen.getByRole("button", { name: "Validar e visualizar" }));
  await waitFor(() => expect(screen.getByText("Evento com imagens")).toBeTruthy());
  expect(screen.getByText(/3 no pacote/)).toBeTruthy();
  expect(upload).toHaveBeenCalledWith("/admin/events/import/package/preview", file);
});

it("requires explicit confirmation before applying a package preview", async () => {
  upload
    .mockResolvedValueOnce({
      manifest: {
        action: "create",
        name: "Pacote aplicável",
        slug: "pacote-aplicavel",
        schemaVersion: 2,
        itemCount: 1,
        optionCount: 2,
        linkCount: 0,
        manifestFingerprint: "manifest",
        baseFingerprint: "base",
        safeChanges: [],
        blockedChanges: [],
      },
      assetCount: 1,
      existingAssetCount: 0,
      addedAssetCount: 1,
    })
    .mockResolvedValueOnce({ result: { action: "create", state: "published" } });
  renderPanel();
  const file = new File([new Uint8Array([80, 75, 3, 4])], "pacote-aplicavel.zip", { type: "application/zip" });
  fireEvent.change(screen.getByLabelText("Manifesto ou pacote"), { target: { files: [file] } });
  fireEvent.click(screen.getByRole("button", { name: "Validar e visualizar" }));
  await waitFor(() => expect(screen.getByText("Pacote aplicável")).toBeTruthy());
  fireEvent.click(screen.getByRole("button", { name: "Aplicar manifesto" }));
  expect(screen.getByRole("button", { name: "Confirmar aplicação" })).toBeTruthy();
  expect(upload).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "Confirmar aplicação" }));
  await waitFor(() => expect(screen.getByText("Versão publicada.")).toBeTruthy());
  expect(upload).toHaveBeenCalledTimes(2);
  expect(upload.mock.calls[1][0]).toBe("/admin/events/import/package/apply");
  expect(post).toHaveBeenCalledWith("/admin/reauth/verify");
  expect(post.mock.invocationCallOrder[0]).toBeLessThan(upload.mock.invocationCallOrder[1]);
});
