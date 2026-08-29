import { Link } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { buttonVariants } from "@/components/ui/button";
import { TermsSections } from "./legal/TermsSections";

export function TermsPage() {
  return (
    <PageShell className="max-w-[820px]">
      <div className="space-y-6">
        <header className="space-y-3">
          <Link to="/" className={buttonVariants({ variant: "link", size: "sm" })}>← Voltar</Link>
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-mint-dark">
            Documentação
          </p>
          <h1 className="text-3xl sm:text-4xl">Termos de Uso do Presumidos</h1>
          <p className="text-sm leading-6 text-ink-muted">
            <span className="font-semibold text-ink">Última atualização:</span> 14 de junho de
            2026
          </p>
          <p className="max-w-3xl text-sm leading-6 text-ink-muted">
            Bem-vindo ao Presumidos. Estes Termos de Uso definem as regras gerais para uso da
            plataforma, incluindo cadastro, participação em bolões, envio de palpites, ranking,
            notificações e demais funcionalidades disponíveis.
          </p>
          <p className="max-w-3xl text-sm leading-6 text-ink-muted">
            Ao acessar ou utilizar o Presumidos, o usuário declara que leu, entendeu e concorda
            com estes Termos de Uso e com a Política de Privacidade da plataforma.
          </p>
        </header>

        <TermsSections />
      </div>
    </PageShell>
  );
}
