CREATE TABLE IF NOT EXISTS cupons (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id UUID NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
    tipo       VARCHAR(10) NOT NULL,             -- '10%' ou '30%'
    codigo     VARCHAR(32) NOT NULL UNIQUE,
    utilizado  BOOLEAN     NOT NULL DEFAULT FALSE,
    criado_em  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cupons_usuario ON cupons(usuario_id);
CREATE INDEX IF NOT EXISTS idx_cupons_codigo ON cupons(codigo);
