-- gen_random_uuid() é nativo do PostgreSQL 13+ (não precisa de superuser/extensão).
CREATE TABLE IF NOT EXISTS usuarios (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nome          VARCHAR(255) NOT NULL,
    telefone      VARCHAR(20)  NOT NULL,
    cpf           VARCHAR(14)  NOT NULL UNIQUE,
    data_cadastro TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_usuarios_cpf ON usuarios(cpf);
