/**
 * meus-cupons.js — consulta os cupons de um participante pelo CPF.
 * Usa o endpoint público GET /api/meus-cupons?cpf=...
 */
(function () {
  'use strict';

  function somenteDigitos(v) {
    return (v || '').replace(/\D/g, '');
  }

  function formatarCpf(v) {
    var d = somenteDigitos(v).slice(0, 11);
    if (d.length > 9) return d.slice(0, 3) + '.' + d.slice(3, 6) + '.' + d.slice(6, 9) + '-' + d.slice(9);
    if (d.length > 6) return d.slice(0, 3) + '.' + d.slice(3, 6) + '.' + d.slice(6);
    if (d.length > 3) return d.slice(0, 3) + '.' + d.slice(3);
    return d;
  }

  function escapar(t) {
    var div = document.createElement('div');
    div.textContent = t == null ? '' : String(t);
    return div.innerHTML;
  }

  /** Copia texto sem diálogos/permissão (funciona em HTTP via execCommand). */
  function copiarSilencioso(texto) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(texto).catch(function () { execCopy(texto); });
      return;
    }
    execCopy(texto);
  }

  function execCopy(texto) {
    try {
      var ta = document.createElement('textarea');
      ta.value = texto;
      ta.setAttribute('readonly', '');
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      ta.setSelectionRange(0, ta.value.length);
      document.execCommand('copy');
      document.body.removeChild(ta);
    } catch (e) { /* silencioso */ }
  }

  document.addEventListener('DOMContentLoaded', function () {
    var form = document.getElementById('form-consulta');
    var input = document.getElementById('consulta-email');
    var alvo = document.getElementById('cupons-resultado');
    if (!form || !input || !alvo) {
      return;
    }

    form.addEventListener('submit', function (e) {
      e.preventDefault();
      var email = (input.value || '').trim();
      if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
        alvo.innerHTML = '<p class="consulta-msg">Digite um e-mail válido.</p>';
        return;
      }

      alvo.innerHTML = '<p class="consulta-msg">Buscando...</p>';

      fetch('/api/meus-cupons?email=' + encodeURIComponent(email))
        .then(function (resp) {
          return resp.json().then(function (dados) {
            return { ok: resp.ok, dados: dados };
          });
        })
        .then(function (r) {
          if (!r.ok) {
            alvo.innerHTML = '<p class="consulta-msg">' + escapar((r.dados && r.dados.erro) || 'Erro na consulta.') + '</p>';
            return;
          }
          var cupons = r.dados || [];
          if (!cupons.length) {
            alvo.innerHTML = '<p class="consulta-msg">Nenhum cupom encontrado para este e-mail.</p>';
            return;
          }
          var html = '<ul class="cupons-lista">';
          for (var i = 0; i < cupons.length; i++) {
            var c = cupons[i];
            html +=
              '<li class="cupom-item' + (c.utilizado ? ' is-usado' : '') + '">' +
              '<span class="cupom-item__tipo">' + escapar(c.tipo) + '</span>' +
              '<span class="cupom-item__codigo">' + escapar(c.codigo) + '</span>' +
              '<button type="button" class="cupom-item__copiar" data-codigo="' + escapar(c.codigo) + '">Copiar</button>' +
              '</li>';
          }
          html += '</ul>';
          alvo.innerHTML = html;

          // Botões de copiar o código (cópia silenciosa, sem diálogos).
          var botoes = alvo.querySelectorAll('.cupom-item__copiar');
          for (var k = 0; k < botoes.length; k++) {
            botoes[k].addEventListener('click', function () {
              var btn = this;
              copiarSilencioso(btn.getAttribute('data-codigo'));
              var original = btn.textContent;
              btn.textContent = 'Copiado!';
              setTimeout(function () { btn.textContent = original; }, 1500);
            });
          }
        })
        .catch(function () {
          alvo.innerHTML = '<p class="consulta-msg">Erro de conexão. Tente novamente.</p>';
        });
    });
  });
})();
