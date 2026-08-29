import { Link } from "react-router-dom";
import { Section } from "./Section";

export function PrivacyDataSections() {
  return (
    <>        <Section title="1. Responsável pelo tratamento dos dados">
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


    </>
  );
}
