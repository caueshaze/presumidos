import { useState } from "react";
import { useSetKnockoutReleased, useReauth } from "@/hooks/queries";
import { withAdminReauth } from "@/lib/adminReauth";
import { Card } from "./ui/card";
import { Button } from "./ui/button";
import { ErrorBanner } from "./ui/field";

export function KnockoutControl({ released }: { released: boolean }) {
  const setReleased = useSetKnockoutReleased();
  const reauth = useReauth();
  const [error, setError] = useState("");

  const toggle = async () => {
    setError("");
    try {
      await withAdminReauth(
        () => setReleased.mutateAsync(!released),
        (password) => reauth.mutateAsync(password),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao alterar a fase.");
    }
  };

  return (
    <Card className="mb-5 border-l-4 border-yellow-dark">
      <h3 className="text-lg">Fases do mata-mata (admin)</h3>
      <p className="mt-1 text-sm text-ink-muted">
        {released
          ? "O mata-mata está liberado e visível para todos os participantes."
          : "O mata-mata está oculto. Você ainda vê todos os jogos para montar os confrontos; libere quando a fase de grupos terminar."}
      </p>
      {error && (
        <div className="mt-3">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}
      <Button
        className="mt-3"
        variant={released ? "outline" : "primary"}
        disabled={setReleased.isPending}
        onClick={toggle}
      >
        {setReleased.isPending
          ? "Salvando..."
          : released
            ? "Ocultar mata-mata"
            : "Liberar mata-mata"}
      </Button>
    </Card>
  );
}
