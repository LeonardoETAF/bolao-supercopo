CREATE TABLE IF NOT EXISTS palpites (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id  UUID NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
    jogo_id     UUID NOT NULL REFERENCES jogos(id) ON DELETE CASCADE,
    gols_time_a SMALLINT NOT NULL,
    gols_time_b SMALLINT NOT NULL,
    pontuacao   SMALLINT NOT NULL DEFAULT 0,
    criado_em   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (usuario_id, jogo_id)
);

CREATE INDEX IF NOT EXISTS idx_palpites_usuario ON palpites(usuario_id);
CREATE INDEX IF NOT EXISTS idx_palpites_jogo ON palpites(jogo_id);
