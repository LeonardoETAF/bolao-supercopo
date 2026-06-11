/**
 * form.js
 * Formulário de palpite do Bolão Super Copo da Copa.
 *
 * Responsabilidades:
 *  - Máscaras de CPF e telefone em tempo real.
 *  - Validação client-side (nome, telefone, CPF com dígitos verificadores).
 *  - Envio via fetch para /api/palpite e exibição do modal de sucesso.
 *  - Funções globais: copiarCupom().
 */
(function () {
  'use strict';

  /* ----------------------------------------------------------------
   * Utilidades de máscara
   * ---------------------------------------------------------------- */

  /**
   * Remove tudo que não for dígito.
   */
  function somenteDigitos(valor) {
    return (valor || '').replace(/\D/g, '');
  }

  /**
   * Formata CPF como 000.000.000-00 (máx. 11 dígitos).
   */
  function formatarCpf(valor) {
    var d = somenteDigitos(valor).slice(0, 11);
    var out = d;
    if (d.length > 9) {
      out = d.slice(0, 3) + '.' + d.slice(3, 6) + '.' + d.slice(6, 9) + '-' + d.slice(9);
    } else if (d.length > 6) {
      out = d.slice(0, 3) + '.' + d.slice(3, 6) + '.' + d.slice(6);
    } else if (d.length > 3) {
      out = d.slice(0, 3) + '.' + d.slice(3);
    }
    return out;
  }

  /**
   * Formata telefone como (00) 00000-0000 (11 díg.) ou (00) 0000-0000 (10 díg.).
   */
  function formatarTelefone(valor) {
    var d = somenteDigitos(valor).slice(0, 11);
    if (d.length === 0) {
      return '';
    }
    if (d.length <= 2) {
      return '(' + d;
    }
    if (d.length <= 6) {
      return '(' + d.slice(0, 2) + ') ' + d.slice(2);
    }
    if (d.length <= 10) {
      // Telefone fixo: (00) 0000-0000
      return '(' + d.slice(0, 2) + ') ' + d.slice(2, 6) + '-' + d.slice(6);
    }
    // Celular: (00) 00000-0000
    return '(' + d.slice(0, 2) + ') ' + d.slice(2, 7) + '-' + d.slice(7);
  }

  /* ----------------------------------------------------------------
   * Validação de CPF (dígitos verificadores)
   * ---------------------------------------------------------------- */

  /**
   * Valida um CPF (string só com dígitos esperada, mas tolera máscara).
   * Rejeita sequências repetidas (ex.: 11111111111).
   */
  function cpfValido(valor) {
    var cpf = somenteDigitos(valor);

    if (cpf.length !== 11) {
      return false;
    }

    // Rejeita todos os dígitos iguais.
    if (/^(\d)\1{10}$/.test(cpf)) {
      return false;
    }

    var i, soma, resto;

    // Primeiro dígito verificador.
    soma = 0;
    for (i = 0; i < 9; i++) {
      soma += parseInt(cpf.charAt(i), 10) * (10 - i);
    }
    resto = (soma * 10) % 11;
    if (resto === 10 || resto === 11) {
      resto = 0;
    }
    if (resto !== parseInt(cpf.charAt(9), 10)) {
      return false;
    }

    // Segundo dígito verificador.
    soma = 0;
    for (i = 0; i < 10; i++) {
      soma += parseInt(cpf.charAt(i), 10) * (11 - i);
    }
    resto = (soma * 10) % 11;
    if (resto === 10 || resto === 11) {
      resto = 0;
    }
    if (resto !== parseInt(cpf.charAt(10), 10)) {
      return false;
    }

    return true;
  }

  /**
   * Validação básica de e-mail (formato local@dominio.tld, sem espaços).
   */
  function emailValido(valor) {
    var e = (valor || '').trim();
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e);
  }

  /* ----------------------------------------------------------------
   * Helpers de erro
   * ---------------------------------------------------------------- */

  function definirErro(spanId, mensagem) {
    var span = document.getElementById(spanId);
    if (span) {
      span.textContent = mensagem || '';
    }
  }

  function limparErros() {
    definirErro('erro-nome', '');
    definirErro('erro-telefone', '');
    definirErro('erro-email', '');
    var geral = document.getElementById('erro-geral');
    if (geral) {
      geral.textContent = '';
    }
  }

  /**
   * Mostra uma mensagem de erro geral (#erro-geral) ou, na falta, um alert.
   */
  function mostrarErroGeral(mensagem) {
    var geral = document.getElementById('erro-geral');
    if (geral) {
      geral.textContent = mensagem;
    } else {
      alert(mensagem);
    }
  }

  /* ----------------------------------------------------------------
   * Inicialização
   * ---------------------------------------------------------------- */

  document.addEventListener('DOMContentLoaded', function () {
    var form = document.getElementById('form-palpite');
    if (!form) {
      // Página sem formulário de palpite.
      return;
    }

    var inputEmail = form.querySelector('[name="email"]');
    var inputTelefone = form.querySelector('[name="telefone"]');
    var inputNome = form.querySelector('[name="nome"]');

    /* ---- Máscaras em tempo real ---- */

    if (inputEmail) {
      inputEmail.addEventListener('input', function () {
        if (emailValido(inputEmail.value)) {
          definirErro('erro-email', '');
        }
      });
    }

    if (inputTelefone) {
      inputTelefone.addEventListener('input', function () {
        inputTelefone.value = formatarTelefone(inputTelefone.value);
        var d = somenteDigitos(inputTelefone.value);
        if (d.length === 10 || d.length === 11) {
          definirErro('erro-telefone', '');
        }
      });
    }

    if (inputNome) {
      inputNome.addEventListener('input', function () {
        if (inputNome.value.trim().length >= 3) {
          definirErro('erro-nome', '');
        }
      });
    }

    /* ---- Validação completa do formulário ---- */

    function validar() {
      limparErros();
      var ok = true;

      // Nome: mínimo 3 caracteres.
      var nome = inputNome ? inputNome.value.trim() : '';
      if (nome.length < 3) {
        definirErro('erro-nome', 'Informe seu nome.');
        ok = false;
      }

      // Telefone: 10 ou 11 dígitos.
      var telDigitos = inputTelefone ? somenteDigitos(inputTelefone.value) : '';
      if (telDigitos.length !== 10 && telDigitos.length !== 11) {
        definirErro('erro-telefone', 'Informe seu telefone.');
        ok = false;
      }

      // E-mail: formato válido.
      var emailVal = inputEmail ? inputEmail.value : '';
      if (!emailValido(emailVal)) {
        definirErro('erro-email', 'E-mail inválido.');
        ok = false;
      }

      // Placar: obrigatório (ambos os campos preenchidos).
      var golsAEl = form.querySelector('[name="gols_time_a"]');
      var golsBEl = form.querySelector('[name="gols_time_b"]');
      var golsAVazio = !golsAEl || golsAEl.value.trim() === '';
      var golsBVazio = !golsBEl || golsBEl.value.trim() === '';
      if (golsAVazio || golsBVazio) {
        mostrarErroGeral('Informe o placar.');
        ok = false;
      }

      return ok;
    }

    /* ---- Envio ---- */

    form.addEventListener('submit', function (evento) {
      evento.preventDefault();

      if (!validar()) {
        return;
      }

      // Monta o payload a partir dos campos do formulário.
      var jogoIdEl = form.querySelector('[name="jogo_id"]');
      var golsAEl = form.querySelector('[name="gols_time_a"]');
      var golsBEl = form.querySelector('[name="gols_time_b"]');

      var payload = {
        nome: inputNome ? inputNome.value.trim() : '',
        telefone: inputTelefone ? somenteDigitos(inputTelefone.value) : '',
        email: inputEmail ? inputEmail.value.trim() : '',
        jogo_id: jogoIdEl ? jogoIdEl.value : '',
        gols_time_a: golsAEl ? Number(golsAEl.value) : 0,
        gols_time_b: golsBEl ? Number(golsBEl.value) : 0
      };

      // Desabilita o botão durante o envio.
      var btn = form.querySelector('[type="submit"]');
      var textoOriginal = '';
      if (btn) {
        textoOriginal = btn.textContent;
        btn.disabled = true;
        btn.textContent = 'Enviando...';
      }

      function reabilitarBotao() {
        if (btn) {
          btn.disabled = false;
          btn.textContent = textoOriginal;
        }
      }

      fetch('/api/palpite', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      })
        .then(function (resp) {
          // Lê o corpo como JSON em qualquer caso (sucesso ou erro).
          return resp.json().then(function (dados) {
            return { ok: resp.ok, dados: dados };
          });
        })
        .then(function (resultado) {
          if (resultado.ok && resultado.dados && resultado.dados.sucesso) {
            // Sucesso: mostra o cupom no modal.
            var cupom = resultado.dados.cupom || {};
            var cupomEl = document.getElementById('cupom-codigo');
            if (cupomEl) {
              cupomEl.textContent = cupom.codigo || '';
            }
            fecharModalPalpite();
            abrirModalSucesso();
            form.reset();
          } else {
            // Erro retornado pela API.
            var msg =
              (resultado.dados && resultado.dados.erro) ||
              'Não foi possível enviar seu palpite. Tente novamente.';
            mostrarErroGeral(msg);
          }
        })
        .catch(function () {
          mostrarErroGeral('Erro de conexão. Verifique sua internet e tente novamente.');
        })
        .then(function () {
          // finally: reabilita o botão sempre.
          reabilitarBotao();
        });
    });
  });

  /* ----------------------------------------------------------------
   * Modal de sucesso
   * ---------------------------------------------------------------- */

  function abrirModalSucesso() {
    var modal = document.getElementById('modal-sucesso');
    if (modal) {
      modal.removeAttribute('hidden');
    }
  }

  function fecharModalSucesso() {
    var modal = document.getElementById('modal-sucesso');
    if (modal) {
      modal.setAttribute('hidden', '');
    }
  }

  /* ---- Modal do formulário de palpite ---- */
  function abrirModalPalpite() {
    var modal = document.getElementById('modal-palpite');
    if (modal) {
      modal.removeAttribute('hidden');
    }
  }

  function fecharModalPalpite() {
    var modal = document.getElementById('modal-palpite');
    if (modal) {
      modal.setAttribute('hidden', '');
    }
  }
  window.abrirModalPalpite = abrirModalPalpite;
  window.fecharModalPalpite = fecharModalPalpite;

  document.addEventListener('DOMContentLoaded', function () {
    var botaoAbrir = document.getElementById('abrir-palpite');
    if (botaoAbrir) {
      botaoAbrir.addEventListener('click', abrirModalPalpite);
    }

    var botaoHeader = document.getElementById('header-participar');
    if (botaoHeader) {
      botaoHeader.addEventListener('click', abrirModalPalpite);
    }

    // Header fixo: mostra fundo e o botão "Participar" ao rolar a tela.
    var siteHeader = document.getElementById('site-header');
    var headerLinks = document.getElementById('header-links');
    if (siteHeader) {
      var aoRolar = function () {
        var rolou = window.scrollY > 200;
        siteHeader.classList.toggle('is-scrolled', rolou);
        // Ao rolar: esconde Como Funciona/Ranking e mostra Participar.
        if (headerLinks) headerLinks.style.display = rolou ? 'none' : 'flex';
        if (botaoHeader) botaoHeader.style.display = rolou ? 'inline-flex' : 'none';
      };
      window.addEventListener('scroll', aoRolar, { passive: true });
      aoRolar();
    }

    var modalPalpite = document.getElementById('modal-palpite');
    if (modalPalpite) {
      modalPalpite.addEventListener('click', function (evento) {
        var card = modalPalpite.querySelector('.palpite-box');
        if (
          evento.target.hasAttribute('data-fechar-palpite') ||
          (card && !card.contains(evento.target))
        ) {
          fecharModalPalpite();
        }
      });
    }
  });

  // O modal de sucesso fecha SOMENTE pelo botão Fechar (não fecha ao clicar fora).
  document.addEventListener('DOMContentLoaded', function () {
    var modal = document.getElementById('modal-sucesso');
    if (!modal) {
      return;
    }
    var botoesFechar = modal.querySelectorAll('[data-fechar-modal]');
    for (var i = 0; i < botoesFechar.length; i++) {
      botoesFechar[i].addEventListener('click', function (evento) {
        evento.preventDefault();
        fecharModalSucesso();
      });
    }
  });

  // Expõe a função de fechar globalmente (para uso em onclick, se houver).
  window.fecharModalSucesso = fecharModalSucesso;

  /* ----------------------------------------------------------------
   * Cópia do cupom
   * ---------------------------------------------------------------- */

  /**
   * Copia o código do cupom (#cupom-codigo) para a área de transferência.
   * Função global chamada via onclick no HTML.
   */
  function copiarCupom() {
    var cupomEl = document.getElementById('cupom-codigo');
    if (!cupomEl) {
      return;
    }
    var codigo = cupomEl.textContent.trim();
    if (!codigo) {
      return;
    }

    var botao = typeof event !== 'undefined' && event ? event.target : null;
    function feedback() {
      if (botao) {
        var original = botao.textContent;
        botao.textContent = 'Copiado!';
        setTimeout(function () { botao.textContent = original; }, 2000);
      }
    }
    function execCopy() {
      try {
        var area = document.createElement('textarea');
        area.value = codigo;
        area.setAttribute('readonly', '');
        area.style.position = 'fixed';
        area.style.left = '-9999px';
        area.style.opacity = '0';
        document.body.appendChild(area);
        area.select();
        document.execCommand('copy');
        document.body.removeChild(area);
      } catch (e) { /* silencioso */ }
    }

    // Cópia silenciosa: sem alerts/diálogos do navegador.
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(codigo).then(feedback, function () {
        execCopy();
        feedback();
      });
    } else {
      execCopy();
      feedback();
    }
  }

  // Expõe globalmente.
  window.copiarCupom = copiarCupom;
})();
