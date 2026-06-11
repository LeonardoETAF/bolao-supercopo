#!/usr/bin/env bash
# Teste de fluxo ponta a ponta da API do Bolão Super Copo.
# Pré-requisito: o servidor precisa estar rodando (cargo run --bin server).
# Uso: bash scripts/smoke_test.sh
set -euo pipefail

B="${BASE_URL:-http://localhost:3000}"
ADMIN_USER="${ADMIN_USER:-admin}"
ADMIN_PASS="${ADMIN_PASS:-supercopo2026}"

jqget() { python3 -c "import sys,json;print(json.load(sys.stdin)['$1'])"; }

echo "== 1) Páginas e estáticos =="
for p in / /ranking /admin /static/css/style.css /static/js/form.js; do
  printf "  %-24s HTTP %s\n" "$p" "$(curl -s -o /dev/null -w '%{http_code}' "$B$p")"
done

echo "== 2) Login admin =="
TOKEN=$(curl -s -X POST "$B/admin/login" -H 'Content-Type: application/json' \
  -d "{\"usuario\":\"$ADMIN_USER\",\"senha\":\"$ADMIN_PASS\"}" | jqget token)
echo "  token: ${TOKEN:0:20}..."
AUTH=(-H "Authorization: Bearer $TOKEN")

echo "== 3) Cadastrar jogo ativo =="
JID=$(curl -s -X POST "$B/admin/jogos" "${AUTH[@]}" -H 'Content-Type: application/json' \
  -d '{"time_a":"Brasil","time_b":"Argentina","data_jogo":"2026-06-20T18:00:00Z","ativo":true}' | jqget id)
echo "  jogo_id: $JID"

echo "== 4) Enviar palpite válido (CPF 529.982.247-25) -> cupom 10% =="
curl -s -X POST "$B/api/palpite" -H 'Content-Type: application/json' \
  -d "{\"nome\":\"Joao Silva\",\"telefone\":\"11999998888\",\"cpf\":\"529.982.247-25\",\"jogo_id\":\"$JID\",\"gols_time_a\":2,\"gols_time_b\":1}"; echo

echo "== 5) CPF inválido (deve falhar) =="
curl -s -X POST "$B/api/palpite" -H 'Content-Type: application/json' \
  -d "{\"nome\":\"Fulano\",\"telefone\":\"11999998888\",\"cpf\":\"111.111.111-11\",\"jogo_id\":\"$JID\",\"gols_time_a\":1,\"gols_time_b\":0}"; echo

echo "== 6) Palpite duplicado (deve dar 409) =="
curl -s -o /dev/null -w '  HTTP %{http_code}\n' -X POST "$B/api/palpite" -H 'Content-Type: application/json' \
  -d "{\"nome\":\"Joao Silva\",\"telefone\":\"11999998888\",\"cpf\":\"529.982.247-25\",\"jogo_id\":\"$JID\",\"gols_time_a\":3,\"gols_time_b\":3}"

echo "== 7) Informar resultado 2x1 (acerto exato -> 10 pts + cupom 30%) =="
curl -s -X PUT "$B/admin/jogos/$JID/resultado" "${AUTH[@]}" -H 'Content-Type: application/json' \
  -d '{"gols_time_a":2,"gols_time_b":1}'; echo

echo "== 8) Ranking final =="
curl -s "$B/api/ranking?page=1"; echo
echo "== 9) Métricas =="
curl -s "$B/admin/metricas" "${AUTH[@]}"; echo
echo "== 10) Admin sem token (deve 401) =="
curl -s -o /dev/null -w '  HTTP %{http_code}\n' "$B/admin/metricas"

echo "OK ✅"
