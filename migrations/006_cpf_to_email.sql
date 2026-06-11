-- Substitui CPF por e-mail no cadastro de usuários.
ALTER TABLE usuarios ADD COLUMN IF NOT EXISTS email TEXT;

-- Converte dados existentes (CPF vira um e-mail placeholder) para satisfazer NOT NULL/UNIQUE.
UPDATE usuarios SET email = cpf || '@sem-email.invalid' WHERE email IS NULL;

ALTER TABLE usuarios ALTER COLUMN email SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'usuarios_email_key'
    ) THEN
        ALTER TABLE usuarios ADD CONSTRAINT usuarios_email_key UNIQUE (email);
    END IF;
END $$;

DROP INDEX IF EXISTS idx_usuarios_cpf;
ALTER TABLE usuarios DROP COLUMN IF EXISTS cpf;
CREATE INDEX IF NOT EXISTS idx_usuarios_email ON usuarios(email);
