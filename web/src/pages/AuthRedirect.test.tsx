import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { expect, it, vi } from "vitest";
import { LoginPage } from "./Login";
import { RegisterPage } from "./Register";

const mocks = vi.hoisted(() => ({
  auth: { user: null, loading: false, applySession: vi.fn() },
  login: { mutateAsync: vi.fn(), isPending: false },
  registerRequest: { mutateAsync: vi.fn(), isPending: false },
  registerConfirm: { mutateAsync: vi.fn(), isPending: false },
}));

vi.mock("@/hooks/useAuth", () => ({ useAuth: () => mocks.auth }));
vi.mock("@/hooks/queries", () => ({
  useLogin: () => mocks.login,
  useRegisterRequest: () => mocks.registerRequest,
  useRegisterConfirm: () => mocks.registerConfirm,
}));

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="location">{location.pathname}{location.search}</span>;
}

function renderLogin(initialEntry: string) {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="*" element={<LocationProbe />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderRegister(initialEntry: string) {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <Routes>
        <Route path="/register" element={<RegisterPage />} />
        <Route path="*" element={<LocationProbe />} />
      </Routes>
    </MemoryRouter>
  );
}

it("returns from login to the same internal invite", async () => {
  mocks.login.mutateAsync.mockResolvedValueOnce({
    user: { id: "u1", username: "user", email: "u@example.com", isAdmin: false },
    csrfToken: "csrf",
  });
  renderLogin("/login?returnTo=%2Fpools%2Fjoin%2FABC123");

  fireEvent.change(screen.getByPlaceholderText("Usuário ou email"), { target: { value: "user" } });
  fireEvent.change(screen.getByPlaceholderText("Senha"), { target: { value: "password" } });
  fireEvent.click(screen.getByRole("button", { name: "Entrar" }));

  await waitFor(() => expect(screen.getByTestId("location").textContent).toBe("/pools/join/ABC123"));
});

it("rejects an external returnTo after login", async () => {
  mocks.login.mutateAsync.mockResolvedValueOnce({
    user: { id: "u2", username: "user", email: "u@example.com", isAdmin: false },
    csrfToken: "csrf",
  });
  renderLogin("/login?returnTo=https%3A%2F%2Fmalicioso.example");

  fireEvent.change(screen.getByPlaceholderText("Usuário ou email"), { target: { value: "user" } });
  fireEvent.change(screen.getByPlaceholderText("Senha"), { target: { value: "password" } });
  fireEvent.click(screen.getByRole("button", { name: "Entrar" }));

  await waitFor(() => expect(screen.getByTestId("location").textContent).toBe("/"));
});

it("returns from signup to the same internal invite", async () => {
  mocks.registerRequest.mutateAsync.mockResolvedValueOnce(undefined);
  mocks.registerConfirm.mutateAsync.mockResolvedValueOnce({
    user: { id: "u3", username: "new-user", email: "new@example.com", isAdmin: false },
    csrfToken: "csrf",
  });
  renderRegister("/register?returnTo=%2Fpools%2Fjoin%2FZX9876");

  fireEvent.change(screen.getByPlaceholderText("Nome de usuário"), { target: { value: "new-user" } });
  fireEvent.change(screen.getByPlaceholderText("Email"), { target: { value: "new@example.com" } });
  fireEvent.change(screen.getByPlaceholderText("Senha"), { target: { value: "password123" } });
  fireEvent.change(screen.getByPlaceholderText("Confirmar senha"), { target: { value: "password123" } });
  fireEvent.click(screen.getByRole("button", { name: "Criar conta" }));

  await waitFor(() => expect(screen.getByPlaceholderText("Código de 6 dígitos")).toBeTruthy());
  fireEvent.change(screen.getByPlaceholderText("Código de 6 dígitos"), { target: { value: "123456" } });
  fireEvent.click(screen.getByRole("button", { name: "Confirmar conta" }));

  await waitFor(() => expect(screen.getByTestId("location").textContent).toBe("/pools/join/ZX9876"));
});
