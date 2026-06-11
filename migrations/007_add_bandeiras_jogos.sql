-- Bandeira (arquivo em static/img) escolhida para cada time no cadastro do jogo.
-- Quando nula, a landing cai no mapeamento por nome do time.
ALTER TABLE jogos ADD COLUMN IF NOT EXISTS bandeira_a TEXT;
ALTER TABLE jogos ADD COLUMN IF NOT EXISTS bandeira_b TEXT;
