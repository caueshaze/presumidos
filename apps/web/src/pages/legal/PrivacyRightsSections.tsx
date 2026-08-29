import { Link } from "react-router-dom";
import { Section } from "./Section";

export function PrivacyRightsSections() {
  return (
    <>        <Section title="8. Cookies e sessão">
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

    </>
  );
}
