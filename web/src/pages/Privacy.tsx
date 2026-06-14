import { Link } from "react-router-dom";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";

function Section({
  id,
  title,
  children,
}: {
  id?: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="scroll-mt-24 space-y-3">
      <h2 className="text-xl">{title}</h2>
      <div className="space-y-3 text-sm leading-6 text-ink-muted">{children}</div>
    </section>
  );
}

export function PrivacyPage() {
  return (
    <PageShell className="max-w-[820px]">
      <div className="space-y-6">
        <header className="space-y-3">
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

        <Card className="space-y-6">
          <Section title="1. Responsável pelo tratamento dos dados">
            <p>
              O Presumidos é mantido por seu responsável operacional para fins recreativos, como
              plataforma de organização de bolões, palpites e ranking entre amigos, colegas,
              comunidades ou grupos privados.
            </p>
            <p>
              Para dúvidas, solicitações relacionadas a dados pessoais, exclusão de conta,
              privacidade ou notificações, o usuário pode entrar em contato pelo canal indicado na
              plataforma.
            </p>
            <p>
              <span className="font-semibold text-ink">Contato:</span>{" "}
              <Link to="/contact" className="font-semibold text-mint-dark hover:underline">
                página de contato
              </Link>
            </p>
          </Section>

          <Section title="2. Quais dados o Presumidos coleta">
            <p>O Presumidos trata principalmente os seguintes dados:</p>
            <ul className="list-disc space-y-2 pl-5">
              <li>dados de cadastro, como nome, nome de usuário, apelido e e-mail;</li>
              <li>credenciais protegidas, como hash de senha, nunca a senha em texto puro;</li>
              <li>
                dados de sessão e autenticação, como tokens de sessão, validade de login e
                informações necessárias para manter o usuário autenticado;
              </li>
              <li>
                dados de uso do bolão, como participação em bolões, palpites, pontuação, ranking,
                histórico de partidas e ajustes administrativos;
              </li>
              <li>
                dados de auditoria e segurança, como registros de ações administrativas, tentativas
                de acesso, alterações relevantes e eventos necessários para proteger a plataforma;
              </li>
              <li>
                dados de notificações web push, como preferência de lembrete, endpoint do
                navegador, chaves técnicas de push e informações básicas do dispositivo ou
                navegador quando disponíveis;
              </li>
              <li>
                dados técnicos básicos, como endereço IP, tipo de navegador, sistema operacional,
                data e horário de acesso, quando necessários para segurança, diagnóstico ou
                funcionamento da plataforma.
              </li>
            </ul>
            <p>
              O Presumidos não solicita dados sensíveis, como informações de saúde, religião,
              opinião política, biometria, orientação sexual ou dados semelhantes. Caso algum
              usuário insira esse tipo de informação indevidamente em campos livres, ela poderá ser
              removida quando identificada.
            </p>
          </Section>

          <Section title="3. Para que os dados são usados">
            <p>
              Os dados são utilizados para permitir o funcionamento normal da plataforma,
              incluindo:
            </p>
            <ul className="list-disc space-y-2 pl-5">
              <li>criar, autenticar e proteger contas de usuário;</li>
              <li>confirmar cadastro, recuperar acesso e enviar mensagens operacionais por e-mail;</li>
              <li>registrar, exibir, calcular e organizar palpites, pontuações e rankings;</li>
              <li>permitir a participação em bolões e grupos privados;</li>
              <li>administrar usuários, permissões e regras internas do bolão;</li>
              <li>
                manter a segurança da plataforma, prevenir abuso, investigar uso indevido e auditar
                ações administrativas;
              </li>
              <li>enviar notificações, lembretes e atualizações quando o usuário autorizar;</li>
              <li>corrigir falhas, melhorar funcionalidades e manter registros necessários para operação do serviço.</li>
            </ul>
            <p>
              O Presumidos busca tratar apenas os dados necessários para sua finalidade recreativa e
              operacional.
            </p>
          </Section>

          <Section title="4. Bases para o tratamento dos dados">
            <p>O tratamento de dados no Presumidos pode ocorrer, conforme o caso, para:</p>
            <ul className="list-disc space-y-2 pl-5">
              <li>
                executar o serviço solicitado pelo usuário, como cadastro, login, participação em
                bolões, palpites e ranking;
              </li>
              <li>cumprir obrigações legais ou regulatórias eventualmente aplicáveis;</li>
              <li>proteger a segurança da plataforma, prevenir fraude, abuso ou acesso indevido;</li>
              <li>atender solicitações do próprio usuário;</li>
              <li>enviar notificações quando houver autorização do usuário;</li>
              <li>manter registros mínimos necessários para operação, auditoria e solução de problemas.</li>
            </ul>
            <p>
              Quando uma funcionalidade depender de autorização específica, como notificações push,
              o usuário poderá conceder ou revogar essa autorização conforme as opções do navegador,
              do dispositivo ou da própria plataforma, quando disponível.
            </p>
          </Section>

          <Section title="5. Onde os dados são armazenados">
            <p>
              Os dados operacionais do Presumidos são armazenados na infraestrutura utilizada pelo
              projeto, incluindo banco de dados da aplicação, serviços de hospedagem e serviços
              técnicos vinculados ao funcionamento da plataforma.
            </p>
            <p>
              Dependendo da infraestrutura contratada, os dados poderão ser armazenados ou
              processados em servidores localizados no Brasil ou em outros países.
            </p>
            <p>
              Dados de verificação, e-mail e notificações podem transitar por serviços externos
              estritamente necessários para envio de mensagens, autenticação, hospedagem, segurança
              e web push.
            </p>
          </Section>

          <Section title="6. Com quem os dados podem ser compartilhados">
            <p>O Presumidos não vende dados pessoais.</p>
            <p>
              O compartilhamento de dados pode ocorrer apenas quando necessário para operar,
              proteger ou administrar o serviço, por exemplo:
            </p>
            <ul className="list-disc space-y-2 pl-5">
              <li>com provedores de hospedagem, banco de dados e infraestrutura técnica;</li>
              <li>
                com provedores de e-mail transacional, para envio de códigos, confirmações,
                recuperação de acesso e mensagens operacionais;
              </li>
              <li>
                com a infraestrutura de web push do navegador ou sistema operacional, quando o
                usuário ativa notificações;
              </li>
              <li>
                com administradores do próprio bolão, quando necessário para gestão de membros,
                permissões, pontuação, ranking ou suporte;
              </li>
              <li>
                quando necessário para cumprir obrigação legal, ordem de autoridade competente ou
                proteger direitos, segurança e integridade da plataforma.
              </li>
            </ul>
            <p>
              Administradores de bolões podem visualizar informações necessárias para organizar o
              grupo, como identificação do participante, palpites, pontuação, ranking e histórico
              relacionado ao bolão.
            </p>
          </Section>

          <Section title="7. Notificações push">
            <p>
              O Presumidos pode solicitar permissão para enviar notificações sobre lembretes de
              palpites, início de jogos, resultados, mudanças no ranking, avisos administrativos e
              atualizações do bolão.
            </p>
            <p>
              As notificações só serão enviadas quando o usuário autorizar, conforme as regras do
              navegador ou dispositivo utilizado.
            </p>
            <p>
              Quando as notificações são ativadas, o sistema armazena os identificadores técnicos
              necessários para entregar mensagens ao navegador autorizado naquele dispositivo, como
              endpoint de push, chaves públicas técnicas e preferências de notificação.
            </p>
            <p>
              O usuário pode desativar as notificações a qualquer momento nas configurações do
              navegador, do dispositivo ou, quando disponível, na própria plataforma.
            </p>
            <p>
              A entrega de notificações pode depender de serviços de terceiros, conexão com a
              internet, permissões do dispositivo, configurações do navegador e compatibilidade do
              sistema operacional. O Presumidos não garante entrega imediata, contínua ou sem
              falhas.
            </p>
          </Section>

          <Section title="8. Cookies e sessão">
            <p>
              O Presumidos pode utilizar cookies e mecanismos técnicos semelhantes para manter o
              usuário autenticado, preservar preferências, melhorar a experiência e garantir o
              funcionamento correto da plataforma.
            </p>
            <p>
              Esses recursos podem armazenar dados como sessão autenticada, preferências de
              interface e informações técnicas necessárias para navegação.
            </p>
            <p>
              O bloqueio desses recursos pelo navegador pode impedir login, notificações ou outras
              funcionalidades da plataforma.
            </p>
          </Section>

          <Section title="9. Segurança dos dados">
            <p>
              O Presumidos adota medidas técnicas e organizacionais compatíveis com sua natureza
              recreativa e enxuta para proteger os dados tratados.
            </p>
            <p>Essas medidas podem incluir:</p>
            <ul className="list-disc space-y-2 pl-5">
              <li>armazenamento de senhas apenas em formato protegido por hash;</li>
              <li>uso de HTTPS em ambiente de produção;</li>
              <li>controle de acesso a áreas administrativas;</li>
              <li>separação entre dados públicos, privados e administrativos;</li>
              <li>registros de auditoria para ações relevantes;</li>
              <li>proteção de tokens, chaves e credenciais fora do código público;</li>
              <li>limitação de acesso aos dados apenas a quem precisa operar ou manter a plataforma.</li>
            </ul>
            <p>
              Apesar dos cuidados adotados, nenhum sistema é totalmente imune a falhas, ataques,
              erros humanos ou indisponibilidades. O usuário também deve proteger sua senha,
              dispositivo e acesso ao e-mail cadastrado.
            </p>
          </Section>

          <Section title="10. Retenção e exclusão de dados">
            <p>
              Os dados serão mantidos enquanto forem necessários para funcionamento da conta,
              participação nos bolões, exibição de ranking, segurança, auditoria, prevenção a
              fraude, suporte ou cumprimento de obrigações aplicáveis.
            </p>
            <p>
              O usuário pode solicitar a exclusão ou desativação da conta pela página autenticada{" "}
              <Link to="/conta" className="font-semibold text-mint-dark hover:underline">
                Conta
              </Link>{" "}
              ou pelo canal de contato indicado.
            </p>
            <p>
              Quando a exclusão for concluída, os dados operacionais principais vinculados à conta
              serão removidos ou desvinculados, e a sessão poderá ser encerrada.
            </p>
            <p>
              Em situações específicas, a exclusão poderá ser limitada, adiada ou bloqueada quando
              houver pendências operacionais, como bolões criados por essa conta, necessidade de
              preservar uma conta administrativa válida, investigação de fraude, registros mínimos
              de segurança, disputas internas do bolão ou obrigação legal.
            </p>
            <p>
              Quando possível, dados históricos de ranking, palpites ou participação poderão ser
              anonimizados ou desvinculados da identificação direta do usuário, em vez de removidos
              integralmente, para preservar a integridade do histórico do bolão.
            </p>
          </Section>

          <Section title="11. Direitos do usuário">
            <p>O usuário pode solicitar, conforme aplicável:</p>
            <ul className="list-disc space-y-2 pl-5">
              <li>confirmação sobre a existência de tratamento de seus dados;</li>
              <li>acesso aos dados pessoais tratados pelo Presumidos;</li>
              <li>correção de dados incompletos, inexatos ou desatualizados;</li>
              <li>
                exclusão ou anonimização de dados desnecessários, excessivos ou tratados em
                desconformidade;
              </li>
              <li>informações sobre compartilhamento de dados;</li>
              <li>revogação de consentimentos concedidos, como permissões relacionadas a notificações;</li>
              <li>informações adicionais sobre esta Política de Privacidade.</li>
            </ul>
            <p>
              As solicitações devem ser enviadas pelo canal de contato indicado na plataforma. O
              Presumidos poderá solicitar informações adicionais para confirmar a identidade do
              solicitante antes de atender determinados pedidos.
            </p>
          </Section>

          <Section title="12. Responsabilidades do usuário">
            <p>O usuário é responsável por:</p>
            <ul className="list-disc space-y-2 pl-5">
              <li>fornecer dados corretos no cadastro;</li>
              <li>manter sua senha em segurança;</li>
              <li>revisar seus próprios palpites antes do fechamento das partidas;</li>
              <li>verificar suas permissões de notificação no navegador ou dispositivo;</li>
              <li>não compartilhar acesso indevido à conta;</li>
              <li>comunicar suspeitas de uso não autorizado ou falhas relevantes.</li>
            </ul>
            <p>
              O Presumidos não se responsabiliza por acessos indevidos causados por
              compartilhamento de senha, perda de acesso ao e-mail, dispositivos comprometidos ou
              condutas do próprio usuário.
            </p>
          </Section>

          <Section title="13. Menores de idade">
            <p>
              O Presumidos é destinado ao uso recreativo por participantes capazes de compreender e
              aceitar seus Termos de Uso e esta Política de Privacidade.
            </p>
            <p>
              Caso o usuário seja menor de idade, o uso deve ocorrer com ciência e autorização de
              seus pais ou responsáveis legais.
            </p>
            <p>
              Se for identificado uso indevido por menor de idade sem autorização adequada, a conta
              poderá ser limitada, suspensa ou removida.
            </p>
          </Section>

          <Section title="14. Alterações nesta Política">
            <p>
              Esta Política de Privacidade pode ser atualizada para refletir mudanças no
              funcionamento do Presumidos, novas funcionalidades, ajustes operacionais, melhorias
              de segurança ou alterações nos serviços utilizados.
            </p>
            <p>
              A versão publicada nesta página será considerada a versão mais atual e passará a
              valer a partir de sua publicação.
            </p>
            <p>O uso contínuo do Presumidos após a atualização indica ciência da nova versão.</p>
          </Section>

          <Section id="contato" title="15. Contato">
            <p>
              Pedidos de acesso, correção, exclusão de conta, revogação de consentimento para
              notificações, dúvidas sobre privacidade ou relatos de uso indevido devem ser enviados
              pelo canal indicado na plataforma.
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
