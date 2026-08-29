import { Link } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { buttonVariants } from "@/components/ui/button";
import { PrivacySections } from "./legal/PrivacySections";

export function PrivacyPage() {
  return (
    <PageShell className="max-w-[820px]">
      <div className="space-y-6">
        <header className="space-y-3">
          <Link to="/" className={buttonVariants({ variant: "link", size: "sm" })}>← Voltar</Link>
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-mint-dark">
            Documentação
          </p>
          <h1 className="text-3xl sm:text-4xl">Política de Privacidade do Presumidos</h1>
          <p className="text-sm leading-6 text-ink-muted">
            <span className="font-semibold text-ink">Última atualização:</span> 14 de junho de
            2026
          </p>
          <p className="max-w-3xl text-sm leading-6 text-ink-muted">
            Esta Política de Privacidade explica, de forma direta, quais dados o Presumidos coleta
            e trata, para quais finalidades eles são utilizados, como são protegidos e como o
            usuário pode solicitar acesso, correção, exclusão ou mais informações.
          </p>
          <p className="max-w-3xl text-sm leading-6 text-ink-muted">
            Ao utilizar o Presumidos, o usuário declara estar ciente desta Política de Privacidade
            e dos Termos de Uso da plataforma.
          </p>
        </header>

        <PrivacySections />
      </div>
    </PageShell>
  );
}
