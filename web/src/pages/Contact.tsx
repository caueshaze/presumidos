import { Mail } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";

type ContactInfoResponse = {
  email: string;
};

export function ContactPage() {
  const { data } = useQuery({
    queryKey: ["contact-info"],
    queryFn: () => api.get<ContactInfoResponse>("/contact"),
    staleTime: 5 * 60 * 1000,
  });

  const contactEmail = (data?.email || __CONTACT_EMAIL__).trim();
  const contactHref = contactEmail
    ? `mailto:${contactEmail}?subject=${encodeURIComponent("Contato Presumidos")}`
    : "";

  return (
    <PageShell className="max-w-[820px]">
      <div className="space-y-6">
        <header className="space-y-3">
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-mint-dark">
            Suporte
          </p>
          <h1 className="text-3xl sm:text-4xl">Contato</h1>
          <p className="max-w-3xl text-sm leading-6 text-ink-muted">
            Use este canal para pedidos sobre conta, exclusão de dados, privacidade, suporte de
            acesso, problemas com notificações ou dúvidas gerais sobre o Presumidos.
          </p>
        </header>

        <Card className="space-y-4">
          <h2 className="text-xl">Fale com o responsável pela plataforma</h2>
          <p className="text-sm leading-6 text-ink-muted">
            Ao abrir o contato, descreva da forma mais objetiva possível o problema ou pedido.
            Se fizer sentido, inclua o nome de usuário, o bolão envolvido e o que aconteceu.
          </p>
          {contactEmail ? (
            <div className="flex flex-wrap gap-3">
              <a href={contactHref} className={cn(buttonVariants({ variant: "primary" }))}>
                <Mail className="h-4 w-4" />
                Abrir contato
              </a>
            </div>
          ) : (
            <div className="rounded-md border border-mint-dark/15 bg-bg px-4 py-4 text-sm text-ink-muted">
              O canal de contato desta instância ainda não foi configurado.
            </div>
          )}
        </Card>
      </div>
    </PageShell>
  );
}
