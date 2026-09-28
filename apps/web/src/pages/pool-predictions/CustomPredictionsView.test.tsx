import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CustomPredictionsView } from "./CustomPredictionsView";

it("opens a member prediction page instead of expanding every prediction", () => {
  const navigate = vi.fn();
  render(
    <CustomPredictionsView
      context={{
        navigate,
        selectedPool: "pool-1",
        currentPool: { name: "Bolão", event: { name: "Evento" } },
        setSelectedPool: vi.fn(),
        pools: { data: [] },
        customMembers: { isLoading: false, data: [{ userId: "user-1", username: "Ana", predictions: [{ itemId: "item-1", title: "Campeã", optionLabel: "Time A", points: 3 }] }] },
        selectedMemberId: null,
        showPoolSelector: false,
      }}
    />,
  );

  expect(screen.queryByText("Time A")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: /Ana/ }));
  expect(navigate).toHaveBeenCalledWith("/pools/pool-1/members/user-1");
});
