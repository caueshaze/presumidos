#!/usr/bin/env node
// Envia o aviso do mata-mata para todos os usuários, via API do Resend.
//
// Lê um CSV (coluna `email`, opcionalmente `username`) e dispara um email por
// destinatário — nunca em massa no mesmo `to`, para não vazar a lista.
//
// Uso (sem instalar Node no host, via Docker):
//   cd /srv/presumidos
//   export $(grep -E '^RESEND_(API_KEY|FROM_EMAIL)=' .env | xargs)
//   docker run --rm \
//     -e RESEND_API_KEY -e RESEND_FROM_EMAIL \
//     -e ONLY -e DRY_RUN \
//     -v /srv/presumidos/backups:/work \
//     -v /srv/presumidos/deploy/notify:/app:ro \
//     node:20-alpine node /app/send-knockout.mjs /work/usuarios.csv
//
// Variáveis de ambiente:
//   RESEND_API_KEY     (obrigatória)
//   RESEND_FROM_EMAIL  remetente, ex.: "Presumidos <no-reply@caueti.com>"
//   DRY_RUN=1          não envia; só lista o que enviaria
//   ONLY=a@b.com       envia apenas para esse endereço (teste); ignora o CSV

import { readFileSync } from "node:fs";

// Remove aspas em volta do valor. `docker run --env-file` (ao contrário do
// docker compose env_file) NÃO tira aspas, então RESEND_FROM_EMAIL="Nome <x@y>"
// chegaria com as aspas literais e quebraria o campo `from` no Resend.
function unquote(value) {
  if (!value) return value;
  const v = value.trim();
  if (v.length >= 2 && ((v[0] === '"' && v.at(-1) === '"') || (v[0] === "'" && v.at(-1) === "'"))) {
    return v.slice(1, -1);
  }
  return v;
}

const API_KEY = unquote(process.env.RESEND_API_KEY);
const FROM = unquote(process.env.RESEND_FROM_EMAIL || process.env.RESEND_FROM);
const DRY_RUN = process.env.DRY_RUN === "1" || process.env.DRY_RUN === "true";
const ONLY = (process.env.ONLY || "").trim();
const CSV_PATH = process.argv[2] || "/work/usuarios.csv";

const SUBJECT = "O mata-mata já está liberado no Presumidos";
const SITE_URL = "https://presumidos.caueti.com";
// Resend (plano padrão) aceita ~2 req/s. 600ms entre envios dá margem.
const DELAY_MS = 600;

if (!API_KEY) {
  console.error("ERRO: RESEND_API_KEY não definida.");
  process.exit(1);
}
if (!FROM) {
  console.error("ERRO: RESEND_FROM_EMAIL não definida.");
  process.exit(1);
}

// --- Template HTML (mesmo tema do email de verificação de cadastro) ----------

function emailHtml() {
  const paragraphs = [
    "A fase de grupos está chegando ao fim e o mata-mata já está liberado no Presumidos.",
    "Antes de tudo, obrigado por participar até aqui. Ver o bolão ganhando vida, com os palpites, erros absurdos e pequenas humilhações, deixou tudo bem mais divertido.",
    "Você já pode entrar, conferir os confrontos disponíveis e registrar seus palpites para a próxima fase. Alguns jogos ainda podem aparecer conforme os classificados forem sendo confirmados, então vale dar uma passada por lá antes dos prazos.",
    "Boa sorte nos palpites. Agora é mata-mata, ou seja: estatística nenhuma impede a humilhação pública.",
  ];
  const body = paragraphs
    .map(
      (p) =>
        `<p style="margin:0 0 16px;color:#2d3a3a;font-size:16px;line-height:1.6">${p}</p>`,
    )
    .join("\n      ");

  return `<div style="margin:0;padding:32px 16px;background:linear-gradient(180deg,#eaf6f0 0%,#fff8e7 100%);font-family:'Segoe UI',Helvetica,Arial,sans-serif">
  <div style="max-width:480px;margin:0 auto;background:#ffffff;border-radius:20px;overflow:hidden;box-shadow:0 4px 20px rgba(45,58,58,0.08)">
    <div style="background:linear-gradient(135deg,#a8e6cf 0%,#a0d2eb 100%);padding:28px 32px;text-align:center">
      <h1 style="margin:0;color:#2d3a3a;font-size:26px;font-weight:700;letter-spacing:0.5px">Presumidos</h1>
      <p style="margin:8px 0 0;color:#2d3a3a;font-size:14px;font-weight:600;opacity:0.8">O mata-mata começou 🏆</p>
    </div>
    <div style="padding:32px">
      <p style="margin:0 0 16px;color:#2d3a3a;font-size:16px;line-height:1.6">Oi!</p>
      ${body}
      <div style="text-align:center;margin:28px 0 8px">
        <a href="${SITE_URL}" style="display:inline-block;background:linear-gradient(135deg,#a8e6cf 0%,#a0d2eb 100%);color:#2d3a3a;text-decoration:none;font-size:16px;font-weight:700;padding:14px 32px;border-radius:14px">Fazer meus palpites</a>
      </div>
      <p style="margin:8px 0 0;color:#6b7a7a;font-size:13px;line-height:1.5;text-align:center">ou acesse <a href="${SITE_URL}" style="color:#5fbf9f">${SITE_URL.replace("https://", "")}</a></p>
      <p style="margin:24px 0 0;color:#2d3a3a;font-size:16px;line-height:1.6">Abraços,<br>Presumidos</p>
    </div>
    <div style="border-top:1px solid #eef2ee;padding:18px 32px;text-align:center">
      <p style="margin:0;color:#9aa6a6;font-size:12px;line-height:1.5">Este é um email automático, por favor não responda.<br>Presumidos &middot; seu bolão entre amigos</p>
    </div>
  </div>
</div>`;
}

// Versão texto puro (fallback / clientes sem HTML).
function emailText() {
  return `Oi!

A fase de grupos está chegando ao fim e o mata-mata já está liberado no Presumidos.

Antes de tudo, obrigado por participar até aqui. Ver o bolão ganhando vida, com os palpites, erros absurdos e pequenas humilhações, deixou tudo bem mais divertido.

Você já pode entrar, conferir os confrontos disponíveis e registrar seus palpites para a próxima fase. Alguns jogos ainda podem aparecer conforme os classificados forem sendo confirmados, então vale dar uma passada por lá antes dos prazos.

Acesse aqui:
${SITE_URL}

Boa sorte nos palpites. Agora é mata-mata, ou seja: estatística nenhuma impede a humilhação pública.

Abraços,
Presumidos`;
}

// --- CSV ---------------------------------------------------------------------

// Parser mínimo: trata aspas duplas e a coluna `email` pelo header.
function parseCsv(text) {
  const lines = text.split(/\r?\n/).filter((l) => l.trim() !== "");
  if (lines.length === 0) return [];
  const splitLine = (line) => {
    const out = [];
    let cur = "";
    let inQuotes = false;
    for (let i = 0; i < line.length; i++) {
      const ch = line[i];
      if (inQuotes) {
        if (ch === '"' && line[i + 1] === '"') {
          cur += '"';
          i++;
        } else if (ch === '"') {
          inQuotes = false;
        } else {
          cur += ch;
        }
      } else if (ch === '"') {
        inQuotes = true;
      } else if (ch === ",") {
        out.push(cur);
        cur = "";
      } else {
        cur += ch;
      }
    }
    out.push(cur);
    return out;
  };
  const header = splitLine(lines[0]).map((h) => h.trim().toLowerCase());
  const emailIdx = header.indexOf("email");
  if (emailIdx === -1) {
    throw new Error(`CSV sem coluna 'email'. Cabeçalho: ${header.join(", ")}`);
  }
  return lines.slice(1).map((line) => {
    const cols = splitLine(line);
    return (cols[emailIdx] || "").trim();
  });
}

const EMAIL_RE = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;

function recipients() {
  if (ONLY) return [ONLY];
  const raw = parseCsv(readFileSync(CSV_PATH, "utf8"));
  // Dedup + validação básica.
  const seen = new Set();
  const valid = [];
  const invalid = [];
  for (const email of raw) {
    const e = email.toLowerCase();
    if (!EMAIL_RE.test(e)) {
      if (email) invalid.push(email);
      continue;
    }
    if (seen.has(e)) continue;
    seen.add(e);
    valid.push(email);
  }
  if (invalid.length) {
    console.warn(`Ignorando ${invalid.length} email(s) inválido(s): ${invalid.join(", ")}`);
  }
  return valid;
}

// --- Envio -------------------------------------------------------------------

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function sendOne(to, html, text) {
  const payload = { from: FROM, to: [to], subject: SUBJECT, html, text };
  for (let attempt = 1; attempt <= 4; attempt++) {
    let res;
    try {
      res = await fetch("https://api.resend.com/emails", {
        method: "POST",
        headers: {
          Authorization: `Bearer ${API_KEY}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      });
    } catch (err) {
      if (attempt === 4) throw err;
      await sleep(1000 * attempt);
      continue;
    }
    if (res.ok) {
      const data = await res.json().catch(() => ({}));
      return data.id || "ok";
    }
    // 429 (rate limit) ou 5xx: espera e tenta de novo.
    if ((res.status === 429 || res.status >= 500) && attempt < 4) {
      await sleep(1500 * attempt);
      continue;
    }
    const body = await res.text().catch(() => "");
    throw new Error(`HTTP ${res.status}: ${body.slice(0, 200)}`);
  }
  throw new Error("falha após retries");
}

async function main() {
  const list = recipients();
  const html = emailHtml();
  const text = emailText();

  console.log(`Remetente: ${FROM}`);
  console.log(`Assunto:   ${SUBJECT}`);
  console.log(`Destinatários: ${list.length}${ONLY ? " (modo ONLY)" : ""}`);
  if (DRY_RUN) {
    console.log("\n--- DRY_RUN: nada será enviado ---");
    list.forEach((e) => console.log(`  -> ${e}`));
    console.log(`\nTotal que seria enviado: ${list.length}`);
    return;
  }

  let ok = 0;
  const failures = [];
  for (let i = 0; i < list.length; i++) {
    const to = list[i];
    try {
      const id = await sendOne(to, html, text);
      ok++;
      console.log(`[${i + 1}/${list.length}] OK   ${to} (${id})`);
    } catch (err) {
      failures.push({ to, error: String(err.message || err) });
      console.error(`[${i + 1}/${list.length}] FALHA ${to}: ${err.message || err}`);
    }
    if (i < list.length - 1) await sleep(DELAY_MS);
  }

  console.log(`\nConcluído: ${ok} enviado(s), ${failures.length} falha(s).`);
  if (failures.length) {
    console.log("Falhas (reenvie com ONLY=<email>):");
    failures.forEach((f) => console.log(`  ${f.to} — ${f.error}`));
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("Erro fatal:", err);
  process.exit(1);
});
