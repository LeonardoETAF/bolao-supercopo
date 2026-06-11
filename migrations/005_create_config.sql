-- Estado global do bolão (chave/valor). Usado para encerrar/reabrir a competição.
CREATE TABLE IF NOT EXISTS config (
    chave TEXT PRIMARY KEY,
    valor TEXT NOT NULL
);

INSERT INTO config (chave, valor) VALUES ('bolao_encerrado', 'false')
ON CONFLICT (chave) DO NOTHING;
