# Bolão da Copa Super Copo — Rust Full-Stack

Landing page interativa de bolão da Copa do Mundo: usuários fazem palpites de placar,
acumulam pontos, sobem no ranking ao vivo e recebem cupons de desconto automaticamente.

**100% Rust.**

| Camada    | Tecnologia                                            |
| --------- | ----------------------------------------------------- |
| Backend   | Axum 0.7 + Tokio + SQLx 0.7                            |
| Templates | Askama 0.12 (renderização server-side)                |
| Frontend  | HTML + CSS puro (tema Copa) + JS vanilla (sem build)  |
| Banco     | PostgreSQL (migrations via SQLx)                      |
| Auth admin| JWT (`jsonwebtoken`)                                   |
| Tempo real| SSE (Server-Sent Events) para ranking ao vivo         |
| Cupons    | Códigos únicos (`rand`) — 10% por palpite, 30% no acerto exato |

## Estrutura

```
.
├── Cargo.toml                 # workspace
├── migrations/                # 001..004 (SQLx)
├── crates/
│   ├── server/                # app Axum (rotas, models, auth, SSE)
│   └── frontend/              # structs + templates Askama
├── static/                    # css/, js/, img/
├── docker-compose.yml
└── Dockerfile
```

## Pré-requisitos

- **Rust** (estável) — já instalado.
- **PostgreSQL** acessível via `DATABASE_URL`.
- Para a opção Docker: **Docker + Docker Compose**.

## Rodar localmente

As migrations são aplicadas automaticamente no startup do servidor.

```bash
cp .env.example .env        # ajuste DATABASE_URL / credenciais admin
cargo run --bin server      # sobe em http://localhost:3000
```

> Neste ambiente o `.env` já aponta para o Postgres local existente
> (`postgres://copa:copa_secret@localhost:5432/copa`).

- Landing / palpite: <http://localhost:3000/>
- Ranking completo: <http://localhost:3000/ranking>
- Painel admin: <http://localhost:3000/admin> (usuário/senha do `.env`)

## Rodar com Docker

Sobe um Postgres dedicado + o app, tudo isolado:

```bash
docker compose up --build
```

App em <http://localhost:3000> · Postgres exposto em `localhost:5433`.

## Fluxo de uso

1. **Admin** (`/admin`) faz login, cadastra um jogo e o deixa **ativo**.
2. **Usuários** (`/`) enviam o palpite (nome, telefone, CPF, placar) → ganham **cupom de 10%**.
3. Admin informa o **resultado** do jogo → a pontuação de todos é recalculada e quem
   acertou o placar exato ganha **cupom de 30%**.
4. O **ranking** (`/ranking` e seção da home) atualiza ao vivo via SSE.

## Regras de pontuação

| Situação                              | Pontos | Cupom |
| ------------------------------------- | ------ | ----- |
| Acerto exato do placar                | 10     | 30%   |
| Acertou apenas o vencedor             | 5      | —     |
| Errou                                 | 0      | —     |
| Por participar (qualquer palpite)     | —      | 10%   |

> O empate só pontua no **acerto exato do placar** (10): palpitar empate sem
> cravar o placar é considerado erro (0). Palpitar empate num jogo que teve
> vencedor — ou vencedor num jogo que empatou — também é erro.

## Endpoints

### Públicos
| Método | Rota                   | Descrição                                  |
| ------ | ---------------------- | ------------------------------------------ |
| GET    | `/`                    | Landing + formulário de palpite            |
| GET    | `/ranking`             | Página de ranking completo                 |
| POST   | `/api/palpite`         | Registra palpite e gera cupom de 10%       |
| GET    | `/api/jogo-ativo`      | Jogo atualmente aberto para palpites (JSON)|
| GET    | `/api/meus-cupons?cpf=`| Lista os cupons de um participante pelo CPF |
| GET    | `/api/ranking?page=N`  | Ranking paginado (100/página)              |
| GET    | `/api/ranking/stream`  | SSE — avisa quando o ranking muda          |

> `POST /api/palpite` aplica **rate-limit por IP** (máx. 5/min) e recusa palpites
> após o horário de início do jogo (`400`).

### Admin (exigem `Authorization: Bearer <jwt>`)
| Método | Rota                            | Descrição                          |
| ------ | ------------------------------- | ---------------------------------- |
| POST   | `/admin/login`                  | Autentica e devolve o JWT (24h)    |
| POST   | `/admin/jogos`                  | Cadastra jogo                      |
| GET    | `/admin/jogos`                  | Lista jogos                        |
| PUT    | `/admin/jogos/:id`              | Edita jogo (times e data)          |
| DELETE | `/admin/jogos/:id`              | Exclui jogo (e palpites em cascata)|
| PUT    | `/admin/jogos/:id/ativar`       | Define o jogo como ativo           |
| PUT    | `/admin/jogos/:id/resultado`    | Informa placar e pontua os palpites|
| GET    | `/admin/metricas`               | Métricas gerais                    |
| GET    | `/admin/cupons?filtro=...`      | Lista cupons (todos/disponíveis/utilizados) |
| PUT    | `/admin/cupons/:id/utilizar`    | Dá baixa no cupom (marca utilizado)|
| PUT    | `/admin/bolao/encerrar`         | Encerra o bolão (congela ranking, mostra pódio na home) |
| PUT    | `/admin/bolao/reabrir`          | Reabre o bolão para novos palpites |

> **Vários jogos abertos:** quando há mais de um jogo ativo, a home mostra 1 em
> destaque (com o formulário completo) e os demais em cards de "Outros jogos
> abertos" — o usuário preenche os dados uma vez e palpita em quantos quiser.
>
> **Encerramento:** com o bolão encerrado, a home exibe o **pódio (top 3)** e
> `POST /api/palpite` passa a recusar palpites (`400`).

## Variáveis de ambiente

| Variável       | Descrição                            |
| -------------- | ------------------------------------ |
| `DATABASE_URL` | URL de conexão do Postgres           |
| `JWT_SECRET`   | Segredo para assinar os tokens admin |
| `ADMIN_USER`   | Usuário do painel admin              |
| `ADMIN_PASS`   | Senha do painel admin                |
| `PORT`         | Porta HTTP (padrão 3000)             |
| `RUST_LOG`     | Nível de log (ex.: `info`)           |

## Testes

```bash
cargo test --workspace   # inclui validação de CPF e cálculo de pontos
```

## Notas de implementação

- Erros retornam JSON padronizado `{ "erro": "mensagem" }`.
- CPF validado com algoritmo completo dos dois dígitos verificadores.
- `UNIQUE(usuario_id, jogo_id)` impede palpite duplicado.
- `gen_random_uuid()` (nativo do PostgreSQL 13+) gera os IDs — sem extensão/superuser.
- Pool de conexões com 20 conexões; índices nas colunas de busca/junção.
