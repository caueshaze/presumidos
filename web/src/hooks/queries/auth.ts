import { useMutation } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AuthResult,
} from "@/types";

// ---- Auth mutations -------------------------------------------------------

export function useLogin() {
  return useMutation({
    mutationFn: (vars: { username: string; password: string }) =>
      api.postPublic<AuthResult>("/auth/login", vars),
  });
}

export function useRegisterRequest() {
  return useMutation({
    mutationFn: (vars: { username: string; email: string; password: string }) =>
      api.postPublic<void>("/auth/register", vars),
  });
}

export function useRegisterConfirm() {
  return useMutation({
    mutationFn: (vars: { email: string; code: string }) =>
      api.postPublic<AuthResult>("/auth/register/confirm", vars),
  });
}

export function usePasswordResetRequest() {
  return useMutation({
    mutationFn: (vars: { email: string }) =>
      api.postPublic<void>("/auth/password-reset", vars),
  });
}

export function usePasswordResetConfirm() {
  return useMutation({
    mutationFn: (vars: { email: string; code: string; newPassword: string }) =>
      api.postPublic<void>("/auth/password-reset/confirm", vars),
  });
}
