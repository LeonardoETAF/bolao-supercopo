-- Vincula cada cupom ao jogo do palpite que o gerou.
-- Permite "promover" o cupom de participação para o de acerto do placar
-- (substituindo o desconto) quando o usuário crava o placar daquele jogo.
ALTER TABLE cupons ADD COLUMN IF NOT EXISTS jogo_id UUID REFERENCES jogos(id) ON DELETE SET NULL;
