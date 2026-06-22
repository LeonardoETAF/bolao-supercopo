/**
 * ranking-sse.js
 * Renderiza o ranking (parcial e completo) e mantém atualizado via SSE.
 *
 * Containers:
 *  - #ranking-live              (home, top 5)
 *  - #ranking-tabela-completa   (página /ranking, página atual)
 *  - #ranking-modal-lista       (modal "Ranking Completo" da home, top 10)
 */
(function () {
  'use strict';

  function escapar(texto) {
    var div = document.createElement('div');
    div.textContent = texto == null ? '' : String(texto);
    return div.innerHTML;
  }

  /** Monta uma linha do ranking no padrão do layout. */
  function linha(p, i) {
    var pos = p.posicao != null ? p.posicao : i + 1;
    var classe = pos >= 1 && pos <= 3 ? ' rk-row--' + pos : '';
    // Último palpite com os países do jogo, ex.: "Brasil 2 x 1 Marrocos".
    var sub = p.ultimo_palpite ? escapar(p.ultimo_palpite) : '&nbsp;';
    var pts = p.total_pontos != null ? p.total_pontos : 0;

    return (
      '<div class="rk-row' + classe + '">' +
      '<span class="rk-pos">' + escapar(pos) + 'º</span>' +
      '<div class="rk-info">' +
      '<span class="rk-nome">' + escapar(p.nome) + '</span>' +
      '<span class="rk-sub">' + sub + '</span>' +
      '</div>' +
      '<span class="rk-pts">' + escapar(pts) + ' <small>pts</small></span>' +
      '</div>'
    );
  }

  /** Renderiza uma lista de participantes no container. */
  function renderizarRanking(participantes, container) {
    if (!container) return;

    if (!participantes || participantes.length === 0) {
      var ph = '';
      for (var n = 1; n <= 5; n++) {
        ph += '<div class="rk-row rk-row--ph"><span class="rk-pos">' + n + 'º</span><div class="rk-info"><span class="rk-nome">—</span></div><span class="rk-pts">0 <small>pts</small></span></div>';
      }
      container.innerHTML = ph;
      return;
    }

    var html = '';
    for (var i = 0; i < participantes.length; i++) {
      html += linha(participantes[i], i);
    }
    container.innerHTML = html;
  }

  window.renderizarRanking = renderizarRanking;

  // ---- Paginação e busca (página /ranking) ----
  var paginaAtual = 1;
  var buscaAtual = '';

  function renderizarPaginacao(dados, completaEl) {
    var el = document.getElementById('paginacao');
    if (!el) return;
    var totalPaginas = (dados && dados.total_pages) || 1;
    var atual = (dados && dados.page) || 1;
    if (totalPaginas <= 1) { el.innerHTML = ''; return; }

    el.innerHTML =
      '<button class="paginacao__btn" data-pg="' + (atual - 1) + '"' + (atual <= 1 ? ' disabled' : '') + '>‹ Anterior</button>' +
      '<span class="paginacao__info">Página ' + atual + ' de ' + totalPaginas + '</span>' +
      '<button class="paginacao__btn" data-pg="' + (atual + 1) + '"' + (atual >= totalPaginas ? ' disabled' : '') + '>Próxima ›</button>';

    var botoes = el.querySelectorAll('.paginacao__btn');
    for (var i = 0; i < botoes.length; i++) {
      botoes[i].addEventListener('click', function () {
        var pg = parseInt(this.getAttribute('data-pg'), 10);
        if (pg >= 1 && pg <= totalPaginas) {
          paginaAtual = pg;
          carregarRanking(null, completaEl);
        }
      });
    }
  }

  function carregarRanking(liveEl, completaEl) {
    var pg = completaEl ? paginaAtual : 1;
    var url = '/api/ranking?page=' + pg;
    if (completaEl && buscaAtual) url += '&q=' + encodeURIComponent(buscaAtual);
    fetch(url)
      .then(function (resp) {
        if (!resp.ok) throw new Error('Falha ao carregar ranking');
        return resp.json();
      })
      .then(function (dados) {
        var participantes = (dados && dados.participantes) || [];
        if (liveEl) renderizarRanking(participantes.slice(0, 5), liveEl);
        if (completaEl) {
          if (participantes.length === 0 && buscaAtual) {
            completaEl.innerHTML =
              '<p class="ranking-vazio">Nenhum participante encontrado para “' +
              escapar(buscaAtual) + '”.</p>';
          } else {
            renderizarRanking(participantes, completaEl);
          }
          renderizarPaginacao(dados, completaEl);
        }
      })
      .catch(function (e) {
        console.error('Erro ao carregar ranking:', e);
      });
  }

  document.addEventListener('DOMContentLoaded', function () {
    var liveEl = document.getElementById('ranking-live');
    var completaEl = document.getElementById('ranking-tabela-completa');

    // "Ver Ranking Completo" agora é um link para a página /ranking.

    // Barra de pesquisa (página /ranking).
    var buscaInput = document.getElementById('ranking-busca-input');
    if (buscaInput && completaEl) {
      var debounce;
      buscaInput.addEventListener('input', function () {
        clearTimeout(debounce);
        var valor = this.value;
        debounce = setTimeout(function () {
          buscaAtual = valor.trim();
          paginaAtual = 1;
          carregarRanking(null, completaEl);
        }, 250);
      });
    }

    if (!liveEl && !completaEl) return;

    carregarRanking(liveEl, completaEl);

    if (typeof EventSource !== 'undefined') {
      try {
        var es = new EventSource('/api/ranking/stream');
        es.onmessage = function () {
          carregarRanking(liveEl, completaEl);
        };
        es.onerror = function () {
          console.warn('Conexão SSE do ranking interrompida; tentando reconectar...');
        };
      } catch (e) {
        console.error('Não foi possível iniciar o SSE do ranking:', e);
      }
    }
  });
})();
