import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AssetUploadControl } from "./AssetUploadControl";

const { upload, post } = vi.hoisted(() => ({ upload: vi.fn(), post: vi.fn() }));

vi.mock("@/lib/api", () => ({ api: { upload, post } }));

const asset = {
  assetId: "asset-1",
  sha256: "a".repeat(64),
  mediaType: "image/webp",
  width: 640,
  height: 320,
  byteSize: 100,
  url: "/media/assets/asset-1/cover",
  variants: {
    thumb: "/media/assets/asset-1/thumb",
    card: "/media/assets/asset-1/card",
    cover: "/media/assets/asset-1/cover",
  },
};

beforeEach(() => {
  upload.mockReset();
  post.mockReset();
  vi.stubGlobal("URL", {
    ...URL,
    createObjectURL: vi.fn(() => "blob:preview"),
    revokeObjectURL: vi.fn(),
  });
});

afterEach(() => vi.unstubAllGlobals());

describe("AssetUploadControl", () => {
  it("shows local preview and success after a valid upload", async () => {
    upload.mockResolvedValue(asset);
    const onChanged = vi.fn();
    const { container } = render(
      <AssetUploadControl
        label="Capa"
        uploadPath="/cover"
        removePath="/cover/remove"
        onChanged={onChanged}
      />,
    );
    const input = container.querySelector("input[type=file]")!;
    const file = new File([new Uint8Array([1, 2, 3])], "cover.png", { type: "image/png" });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => expect(screen.getByText("Imagem atualizada")).toBeTruthy());
    expect(container.querySelector("img")).toBeNull();
    expect(upload).toHaveBeenCalledWith("/cover", file);
    expect(onChanged).toHaveBeenCalledWith(asset);
  });

  it("keeps the upload pending and prevents a duplicate submission", async () => {
    let resolveUpload!: (value: typeof asset) => void;
    upload.mockReturnValue(new Promise((resolve) => { resolveUpload = resolve; }));
    const { container } = render(
      <AssetUploadControl
        label="Capa"
        uploadPath="/cover"
        removePath="/cover/remove"
        onChanged={vi.fn()}
      />,
    );
    const input = container.querySelector("input[type=file]")!;
    const file = new File([new Uint8Array([1, 2, 3])], "cover.png", { type: "image/png" });
    fireEvent.change(input, { target: { files: [file] } });
    expect((screen.getByRole("button", { name: "Enviando…" }) as HTMLButtonElement).disabled).toBe(true);
    expect(upload).toHaveBeenCalledTimes(1);
    resolveUpload(asset);
    await waitFor(() => expect(screen.getByText("Imagem atualizada")).toBeTruthy());
  });

  it("replaces an existing internal image after the backend accepts the new file", async () => {
    upload.mockResolvedValue(asset);
    const onChanged = vi.fn();
    const { container } = render(
      <AssetUploadControl
        label="Capa"
        currentUrl="/media/assets/old/cover"
        uploadPath="/cover"
        removePath="/cover/remove"
        onChanged={onChanged}
      />,
    );
    const input = container.querySelector("input[type=file]")!;
    const file = new File([new Uint8Array([4, 5, 6])], "new-cover.webp", { type: "image/webp" });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => expect(screen.getByText("Imagem atualizada")).toBeTruthy());
    expect(onChanged).toHaveBeenCalledWith(asset);
  });

  it("rejects an unsupported file before sending it", () => {
    const { container } = render(
      <AssetUploadControl
        label="Capa"
        uploadPath="/cover"
        removePath="/cover/remove"
        onChanged={vi.fn()}
      />,
    );
    const input = container.querySelector("input[type=file]")!;
    fireEvent.change(input, {
      target: { files: [new File(["<svg/>"], "cover.svg", { type: "image/svg+xml" })] },
    });
    expect(screen.getByText("Use JPEG, PNG ou WebP.")).toBeTruthy();
    expect(upload).not.toHaveBeenCalled();
  });

  it("removes an existing asset and returns to the external/text fallback", async () => {
    post.mockResolvedValue(undefined);
    const onChanged = vi.fn();
    render(
      <AssetUploadControl
        label="Capa"
        currentUrl="https://example.test/cover.jpg"
        uploadPath="/cover"
        removePath="/cover/remove"
        onChanged={onChanged}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Remover" }));
    await waitFor(() => expect(screen.getByText("Imagem removida")).toBeTruthy());
    expect(post).toHaveBeenCalledWith("/cover/remove");
    expect(onChanged).toHaveBeenCalledWith(null);
  });

  it("falls back from a broken internal asset to the external URL", () => {
    const { container } = render(
      <AssetUploadControl
        label="Capa"
        currentUrl="/media/assets/internal/cover"
        fallbackUrl="https://example.test/cover.jpg"
        uploadPath="/cover"
        removePath="/cover/remove"
        onChanged={vi.fn()}
      />,
    );
    const image = container.querySelector("img")!;
    fireEvent.error(image);
    expect(container.querySelector("img")?.getAttribute("src")).toBe("https://example.test/cover.jpg");
  });
});
