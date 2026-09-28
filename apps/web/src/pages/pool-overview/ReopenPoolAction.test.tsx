import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ReopenPoolAction } from "./ReopenPoolAction";

const mutateAsync = vi.fn();

vi.mock("@/hooks/queries", () => ({
  useReopenPool: () => ({ mutateAsync, isPending: false, error: null }),
}));

it("asks for confirmation before reopening the pool", async () => {
  mutateAsync.mockResolvedValue(undefined);
  render(<ReopenPoolAction poolId="pool-1" poolName="Bolão da firma" />);

  fireEvent.click(screen.getByRole("button", { name: "Reabrir bolão" }));
  const dialog = screen.getByRole("dialog", { name: "Reabrir bolão?" });
  expect(screen.getByText("Bolão da firma")).toBeTruthy();

  fireEvent.click(within(dialog).getByRole("button", { name: "Reabrir bolão" }));
  await waitFor(() => expect(mutateAsync).toHaveBeenCalledWith("pool-1"));
});
