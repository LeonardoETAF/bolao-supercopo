CREATE TABLE IF NOT EXISTS jogos (
    id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    time_a    VARCHAR(100) NOT NULL,
    time_b    VARCHAR(100) NOT NULL,
    data_jogo TIMESTAMPTZ  NOT NULL,
    placar_a  SMALLINT,
    placar_b  SMALLINT,
    -- status: 'agendado' | 'ativo' | 'encerrado'
    status    VARCHAR(20)  NOT NULL DEFAULT 'agendado',
    ativo     BOOLEAN      NOT NULL DEFAULT FALSE,
    criado_em TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_jogos_ativo ON jogos(ativo);
