import { Link } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-3">
      <h2 className="text-xl">{title}</h2>
      <div className="space-y-3 text-sm leading-6 text-ink-muted">{children}</div>
    </section>
  );
}

export function TermsPage() {
  return (
    <PageShell className="max-w-[820px]">
      <div className="space-y-6">
        <header className="space-y-3">
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

        <Card className="space-y-6">
          <Section title="1. Natureza do Presumidos">
            <p>
              O Presumidos é uma plataforma recreativa destinada à organização de bolões entre
              amigos, colegas, comunidades ou grupos privados. O serviço tem como finalidade
              facilitar o registro de palpites, acompanhamento de partidas, pontuação e ranking de
              desempenho dos participantes.
            </p>
            <p>
              O Presumidos não intermedeia apostas em dinheiro, não opera jogos de azar, não
              organiza atividade de apostas, não garante premiação financeira e não assume
              obrigação de pagar prêmio, recompensa, indenização ou qualquer valor aos
              participantes.
            </p>
            <p>
              Eventuais prêmios simbólicos, brincadeiras internas, combinações entre participantes
              ou recompensas definidas por terceiros são de responsabilidade exclusiva dos
              organizadores ou participantes envolvidos, não sendo obrigação do Presumidos.
            </p>
          </Section>

          <Section title="2. Cadastro e acesso">
            <p>
              Para utilizar as funções principais da plataforma, o usuário poderá precisar criar
              uma conta e fornecer informações verdadeiras, atualizadas e suficientes para
              identificação dentro do bolão, como nome, apelido, e-mail ou outros dados
              solicitados pela plataforma.
            </p>
            <p>
              Cada conta é pessoal e deve ser utilizada apenas pelo próprio usuário. O usuário é
              responsável por manter a confidencialidade de sua senha e por todas as ações
              realizadas a partir de seu acesso.
            </p>
            <p>
              Caso suspeite de uso indevido, acesso não autorizado ou comprometimento da conta, o
              usuário deve comunicar o responsável pela plataforma assim que possível.
            </p>
            <p>
              O Presumidos poderá recusar, limitar, suspender ou remover cadastros quando houver
              suspeita de fraude, abuso, violação destes Termos ou necessidade de preservar a
              segurança e a integridade da plataforma.
            </p>
          </Section>

          <Section title="3. Regras dos bolões, palpites e ranking">
            <p>
              O usuário é responsável por conferir, preencher e salvar seus próprios palpites
              dentro dos prazos definidos pela plataforma ou pelas regras específicas do bolão.
            </p>
            <p>
              Palpites não salvos, incompletos, inválidos, duplicados, enviados após o fechamento
              da partida ou registrados fora das condições definidas não precisam ser considerados
              válidos.
            </p>
            <p>
              As regras específicas de cada bolão, grupo ou competição podem complementar estes
              Termos, especialmente quanto a pontuação, prazos, critérios de desempate, correções
              administrativas, premiações simbólicas e organização interna.
            </p>
            <p>
              O ranking, a pontuação e a classificação podem ser alterados caso haja correção de
              resultado, revisão de dados oficiais, ajuste administrativo permitido pelas regras do
              bolão, correção de erro operacional, falha técnica ou retificação de informações
              anteriormente exibidas.
            </p>
            <p>
              Resultados exibidos ao vivo, lembretes, notificações, horários, placares e
              informações similares podem sofrer atraso, indisponibilidade temporária ou divergência
              em relação à fonte oficial. A fonte oficial da competição ou partida prevalece em
              caso de divergência relevante.
            </p>
            <p>
              Para exibir placares, status e informações de partidas, o Presumidos pode consultar
              fontes públicas de terceiros, de forma automatizada e em regime de melhor esforço.
              Esses dados são meramente informativos, podem conter atrasos, erros, alterações ou
              indisponibilidade, não são oficiais e não substituem a fonte oficial da competição. O
              administrador do bolão pode corrigir ou ajustar resultados a qualquer momento, e a
              correção administrativa prevalece sobre os dados obtidos automaticamente.
            </p>
          </Section>

          <Section title="4. Conduta do usuário">
            <p>
              O usuário se compromete a utilizar o Presumidos de forma adequada, respeitosa e
              compatível com sua finalidade recreativa.
            </p>
            <p>Não é permitido:</p>
            <ul className="list-disc space-y-2 pl-5">
              <li>
                tentar acessar contas, áreas administrativas, sistemas, servidores ou dados sem
                autorização;
              </li>
              <li>
                usar a plataforma para fraude, manipulação de ranking, abuso de regras ou obtenção
                de vantagem indevida;
              </li>
              <li>explorar falhas técnicas, bugs ou inconsistências da plataforma;</li>
              <li>
                praticar assédio, ameaça, ofensa, discriminação ou qualquer conduta abusiva contra
                outros usuários;
              </li>
              <li>inserir conteúdo ofensivo, ilegal, discriminatório, enganoso ou prejudicial;</li>
              <li>
                utilizar automações, scripts, scraping, ataques, sobrecarga intencional ou
                qualquer prática que prejudique a estabilidade da plataforma;
              </li>
              <li>compartilhar acesso administrativo sem autorização;</li>
              <li>
                utilizar o Presumidos para finalidade diferente daquela prevista nestes Termos.
              </li>
            </ul>
          </Section>

          <Section title="5. Moderação e medidas administrativas">
            <p>
              O Presumidos poderá limitar, suspender, remover ou bloquear usuários, contas,
              palpites, rankings, grupos, permissões administrativas ou acessos quando houver
              violação destes Termos, suspeita de fraude, comportamento abusivo, risco à
              segurança, erro operacional ou necessidade de preservar o funcionamento adequado do
              serviço.
            </p>
            <p>
              Medidas administrativas poderão ser aplicadas sem aviso prévio quando a situação
              exigir resposta imediata para proteger a plataforma, os usuários ou a integridade do
              bolão.
            </p>
            <p>
              Sempre que razoável, o usuário poderá solicitar esclarecimentos pelo canal de contato
              indicado pela plataforma.
            </p>
          </Section>

          <Section title="6. Disponibilidade do serviço">
            <p>
              O Presumidos é oferecido em regime de melhor esforço, como produto recreativo e
              enxuto. Não há garantia de disponibilidade contínua, funcionamento ininterrupto,
              ausência de erros ou operação 24 horas por dia, 7 dias por semana.
            </p>
            <p>
              A plataforma poderá passar por manutenção, atualização, alteração de funcionalidades,
              correção de falhas, indisponibilidade técnica ou interrupção temporária sem aviso
              prévio.
            </p>
            <p>
              O Presumidos também poderá depender de serviços de terceiros, como hospedagem, banco
              de dados, provedores de autenticação, serviços de e-mail, APIs esportivas,
              navegadores e sistemas de notificação. Falhas, limitações ou mudanças nesses serviços
              podem afetar o funcionamento da plataforma.
            </p>
          </Section>

          <Section title="7. Notificações e comunicações">
            <p>
              O Presumidos poderá oferecer notificações, lembretes ou avisos relacionados ao
              bolão, como prazos para palpites, início de partidas, resultados, mudanças no
              ranking, atualizações da plataforma ou informações administrativas.
            </p>
            <p>
              O envio de notificações depende de autorização do usuário, compatibilidade do
              navegador, configurações do dispositivo, conexão com a internet e funcionamento de
              serviços de terceiros.
            </p>
            <p>
              O usuário pode desativar notificações a qualquer momento nas configurações do
              navegador, do dispositivo ou, quando disponível, na própria plataforma.
            </p>
            <p>
              O Presumidos não garante que notificações serão entregues em tempo real, sem atraso
              ou em todas as situações.
            </p>
          </Section>

          <Section title="8. Privacidade e dados pessoais">
            <p>
              O tratamento de dados pessoais realizado pelo Presumidos segue a{" "}
              <Link to="/privacy" className="font-semibold text-mint-dark hover:underline">
                Política de Privacidade
              </Link>
              , que faz parte desta documentação.
            </p>
            <p>
              A Política de Privacidade explica quais dados podem ser coletados, para quais
              finalidades são utilizados, por quanto tempo podem ser mantidos, com quem podem ser
              compartilhados e como o usuário pode solicitar informações, correção ou exclusão de
              dados.
            </p>
            <p>
              Ao utilizar o Presumidos, o usuário também concorda com a Política de Privacidade
              vigente.
            </p>
          </Section>

          <Section title="9. Encerramento de conta">
            <p>
              O usuário pode solicitar a desativação ou exclusão de sua conta pela página{" "}
              <Link to="/conta" className="font-semibold text-mint-dark hover:underline">
                Conta
              </Link>
              , quando estiver autenticado na plataforma.
            </p>
            <p>
              A exclusão pode ser bloqueada em situações específicas necessárias para preservar a
              consistência operacional do serviço, como quando a conta ainda criou bolões ativos ou
              quando for a última conta administradora válida da instância.
            </p>
            <p>
              Algumas informações poderão ser mantidas pelo tempo necessário para preservar o
              histórico do bolão, prevenir fraude, resolver disputas, corrigir erros, cumprir
              obrigações aplicáveis ou manter registros mínimos de segurança e operação, conforme
              descrito na Política de Privacidade.
            </p>
            <p>
              O encerramento da conta pode impedir o acesso a funcionalidades, histórico, ranking,
              palpites e demais recursos vinculados ao usuário.
            </p>
          </Section>

          <Section title="10. Limites de responsabilidade">
            <p>
              Dentro dos limites permitidos, o Presumidos não se responsabiliza por perdas
              indiretas, expectativas de ganho, frustração de premiação, disputas entre
              participantes, combinações feitas fora da plataforma, prêmios prometidos por
              terceiros, falhas de conexão, bloqueios do navegador, indisponibilidade de serviços
              externos, erros causados pelo próprio usuário ou uso inadequado da plataforma.
            </p>
            <p>
              O Presumidos também não se responsabiliza por decisões tomadas pelos usuários com
              base em resultados parciais, notificações, lembretes, rankings provisórios ou
              informações exibidas de forma temporária no sistema.
            </p>
            <p>
              O usuário reconhece que a plataforma tem finalidade recreativa e que pode passar por
              ajustes, correções e indisponibilidades.
            </p>
          </Section>

          <Section title="11. Alterações na plataforma">
            <p>
              O Presumidos poderá alterar, adicionar, remover ou limitar funcionalidades a qualquer
              momento, especialmente para corrigir falhas, melhorar a experiência, adaptar regras
              do bolão, preservar a segurança ou simplificar a operação do serviço.
            </p>
            <p>
              Funcionalidades em teste, experimentais ou recém-lançadas podem apresentar
              instabilidade, erros ou mudanças de comportamento.
            </p>
          </Section>

          <Section title="12. Atualizações destes Termos">
            <p>
              Estes Termos de Uso podem ser atualizados para refletir mudanças no funcionamento da
              plataforma, requisitos operacionais, melhorias de segurança, alterações nas regras
              dos bolões ou ajustes de organização.
            </p>
            <p>
              A versão publicada nesta página será considerada a versão mais atual. O uso contínuo
              do Presumidos após a publicação de alterações indica concordância com os Termos
              atualizados.
            </p>
          </Section>

          <Section title="13. Contato">
            <p>
              Em caso de dúvidas, solicitações, relato de problemas, suspeita de uso indevido ou
              pedidos relacionados à conta, o usuário pode entrar em contato pelo canal indicado na
              plataforma.
            </p>
            <p>
              <span className="font-semibold text-ink">Contato:</span>{" "}
              <Link to="/contact" className="font-semibold text-mint-dark hover:underline">
                página de contato
              </Link>
            </p>
          </Section>
        </Card>
      </div>
    </PageShell>
  );
}
