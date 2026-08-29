import { useEffect, useState, type SyntheticEvent } from "react";
import { ArrowRight, Users } from "lucide-react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { useJoinPool, usePublicPoolInvitePreview } from "@/hooks/queries";
import { authReturnTo } from "@/lib/navigation";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ErrorState, LoadingState } from "@/components/ui/states";
import { PageShell } from "@/components/PageShell";

function formatDeadline(value: string | null): string | null {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("pt-BR", { dateStyle: "medium", timeStyle: "short" });
}

export function PoolInvitePage() {
  const { token = "" } = useParams<{ token: string }>();
  const location = useLocation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const preview = usePublicPoolInvitePreview(token);
  const join = useJoinPool();
  const [imageIndex, setImageIndex] = useState(0);
  const data = preview.data;

  useEffect(() => {
    const previousTitle = document.title;
    document.title = data?.joinStatus !== "invalid" && data?.poolName
      ? `${data.poolName} — Presumidos`
      : "Convite de bolão — Presumidos";
    return () => {
      document.title = previousTitle;
    };
  }, [data]);

  if (preview.isLoading) {
    return <PageShell className="max-w-[680px]"><LoadingState label="Carregando convite..." /></PageShell>;
  }
  if (preview.isError) {
    return <PageShell className="max-w-[680px]"><ErrorState onRetry={() => void preview.refetch()}>Não foi possível carregar o convite agora.</ErrorState></PageShell>;
  }

  if (!data || data.joinStatus === "invalid") {
    return (
      <PageShell className="max-w-[680px]">
        <Card className="p-7 text-center sm:p-10">
          <h1 className="text-2xl">Este convite não é válido</h1>
          <p className="mt-2 text-ink-muted">O link não está disponível ou não existe mais.</p>
        </Card>
      </PageShell>
    );
  }

  const imageSources = [data.coverAssetUrl, data.coverUrl].filter(
    (source): source is string => Boolean(source),
  );
  const image = imageSources[imageIndex];
  const returnTo = `${location.pathname}${location.search}`;
  const deadline = formatDeadline(data.lockDeadline);
  const joinable = data.joinStatus === "joinable";
  const alreadyMember = data.joinStatus === "already_member";

  const onJoin = async () => {
    if (!user) {
      navigate(authReturnTo(returnTo));
      return;
    }
    try {
      const pool = await join.mutateAsync(token);
      navigate(`/pools/${pool.id}`, { replace: true });
    } catch {
      // O erro da mutation aparece abaixo sem revelar detalhes internos.
    }
  };

  const onImageError = (_event: SyntheticEvent<HTMLImageElement>) => {
    setImageIndex((current) => current + 1);
  };

  return (
    <PageShell className="max-w-[680px]">
      <Card className="overflow-hidden p-0 shadow-card">
        {image && imageIndex < imageSources.length && (
          <img
            src={image}
            alt={data.eventName ? `Capa de ${data.eventName}` : "Capa do evento"}
            className="aspect-[2/1] w-full object-cover"
            onError={onImageError}
          />
        )}
        <div className="p-6 sm:p-8">
          <p className="text-sm font-semibold uppercase tracking-[0.16em] text-mint-dark">Convite para um bolão</p>
          <h1 className="mt-2 text-3xl">{data.poolName}</h1>
          <p className="mt-2 text-lg font-semibold text-ink-muted">{data.eventName}</p>
          {data.creatorDisplayName && <p className="mt-5">{data.creatorDisplayName} convidou você para participar.</p>}
          {data.eventDescription && <p className="mt-3 whitespace-pre-line text-ink-muted">{data.eventDescription}</p>}

          <div className="mt-6 grid gap-3 text-sm text-ink-muted sm:grid-cols-2">
            <div className="flex items-center gap-2"><Users className="h-4 w-4 text-mint-dark" /> {data.memberCount} participante(s)</div>
            {deadline && <div>Palpites até {deadline}</div>}
          </div>

          {alreadyMember ? (
            <div className="mt-7">
              <p className="font-semibold">Você já participa deste bolão.</p>
              {data.poolId && <Button className="mt-3 w-full justify-center" onClick={() => navigate(`/pools/${data.poolId}`)}>Abrir bolão <ArrowRight className="h-4 w-4" /></Button>}
            </div>
          ) : joinable ? (
            <div className="mt-7">
              <Button className="w-full justify-center" onClick={() => void onJoin()} disabled={join.isPending}>
                {join.isPending ? "Entrando..." : "Entrar neste bolão"} <ArrowRight className="h-4 w-4" />
              </Button>
              {join.isError && <p className="mt-3 text-sm font-semibold text-red-700">{(join.error as Error).message}</p>}
            </div>
          ) : (
            <p className="mt-7 rounded-2xl bg-mint/20 p-4 font-semibold">Este bolão não aceita mais participantes.</p>
          )}
        </div>
      </Card>
    </PageShell>
  );
}
