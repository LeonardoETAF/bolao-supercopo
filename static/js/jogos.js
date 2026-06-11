/**
 * jogos.js — palpite rápido nos cards de "Outros jogos abertos".
 * Reaproveita os dados de identidade (nome/telefone/CPF/consentimento) do
 * formulário principal (#form-palpite) e envia um palpite por card.
 */
(function () {
  'use strict';

  function somenteDigitos(v) {
    return (v || '').replace(/\D/g, '');
  }

  function cpfValido(valor) {
    var cpf = somenteDigitos(valor);
    if (cpf.length !== 11 || /^(\d)\1{10}$/.test(cpf)) return false;
    var i, soma, resto;
    soma = 0;
    for (i = 0; i < 9; i++) soma += parseInt(cpf.charAt(i), 10) * (10 - i);
    resto = (soma * 10) % 11; if (resto >= 10) resto = 0;
    if (resto !== parseInt(cpf.charAt(9), 10)) return false;
    soma = 0;
    for (i = 0; i < 10; i++) soma += parseInt(cpf.charAt(i), 10) * (11 - i);
    resto = (soma * 10) % 11; if (resto >= 10) resto = 0;
    return resto === parseInt(cpf.charAt(10), 10);
  }

  /** Lê e valida a identidade do formulário principal. */
  function lerIdentidade() {
    var form = document.getElementById('form-palpite');
    if (!form) return { erro: 'Formulário de dados não encontrado.' };

    var nome = (form.querySelector('[name="nome"]') || {}).value || '';
    var tel = (form.querySelector('[name="telefone"]') || {}).value || '';
    var cpf = (form.querySelector('[name="cpf"]') || {}).value || '';
    var consent = document.getElementById('consent');

    nome = nome.trim();
    if (nome.length < 3) return { erro: 'Preencha seu nome no formulário acima.' };
    if ([10, 11].indexOf(somenteDigitos(tel).length) === -1)
      return { erro: 'Preencha um telefone válido no formulário acima.' };
    if (!cpfValido(cpf)) return { erro: 'Preencha um CPF válido no formulário acima.' };
    if (consent && !consent.checked)
      return { erro: 'Aceite a Política de Privacidade no formulário acima.' };

    return { nome: nome, telefone: somenteDigitos(tel), cpf: somenteDigitos(cpf) };
  }

  function abrirModalComCupom(codigo) {
    var cupomEl = document.getElementById('cupom-codigo');
    if (cupomEl) cupomEl.textContent = codigo || '';
    var modal = document.getElementById('modal-sucesso');
    if (modal) modal.removeAttribute('hidden');
  }

  function enviar(card) {
    var msg = card.querySelector('.jogo-card__msg');
    var btn = card.querySelector('.jogo-card__btn');
    if (msg) msg.textContent = '';

    var id = lerIdentidade();
    if (id.erro) {
      if (msg) {
        msg.textContent = id.erro;
        msg.classList.add('is-erro');
      }
      return;
    }

    var payload = {
      nome: id.nome,
      telefone: id.telefone,
      cpf: id.cpf,
      jogo_id: card.getAttribute('data-jogo-id'),
      gols_time_a: Number((card.querySelector('[data-gols-a]') || {}).value || 0),
      gols_time_b: Number((card.querySelector('[data-gols-b]') || {}).value || 0)
    };

    if (btn) { btn.disabled = true; btn.textContent = 'Enviando...'; }

    fetch('/api/palpite', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
      .then(function (resp) {
        return resp.json().then(function (d) { return { ok: resp.ok, dados: d }; });
      })
      .then(function (r) {
        if (r.ok && r.dados && r.dados.sucesso) {
          if (msg) { msg.textContent = 'Palpite registrado! ✅'; msg.classList.remove('is-erro'); }
          card.classList.add('is-feito');
          if (btn) btn.textContent = 'Palpitado ✓';
          if (r.dados.cupom) abrirModalComCupom(r.dados.cupom.codigo);
        } else {
          if (msg) { msg.textContent = (r.dados && r.dados.erro) || 'Erro ao enviar.'; msg.classList.add('is-erro'); }
          if (btn) { btn.disabled = false; btn.textContent = 'Palpitar'; }
        }
      })
      .catch(function () {
        if (msg) { msg.textContent = 'Erro de conexão.'; msg.classList.add('is-erro'); }
        if (btn) { btn.disabled = false; btn.textContent = 'Palpitar'; }
      });
  }

  document.addEventListener('DOMContentLoaded', function () {
    var cards = document.querySelectorAll('.jogo-card');
    for (var i = 0; i < cards.length; i++) {
      (function (card) {
        var btn = card.querySelector('.jogo-card__btn');
        if (btn) btn.addEventListener('click', function () { enviar(card); });
      })(cards[i]);
    }
  });
})();
