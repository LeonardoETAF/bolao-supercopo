/**
 * modal-historico.js
 * Faz o botão "Voltar" (especialmente no mobile) FECHAR o modal aberto, em vez
 * de sair da página. Funciona com qualquer elemento `.modal` cujo estado de
 * abertura é o atributo `hidden` (removido = aberto, presente = fechado).
 *
 * Como funciona:
 *  - Quando um modal abre, empurramos um estado no histórico (pushState).
 *  - Ao tocar em Voltar, o evento `popstate` dispara e fechamos o(s) modal(is),
 *    permanecendo na mesma página.
 *  - Ao fechar pela própria interface (X, clicar fora), desfazemos o estado que
 *    empurramos (history.back) para que o próximo Voltar saia normalmente.
 *
 * Um MutationObserver observa o atributo `hidden`, então não importa qual script
 * abre/fecha o modal — a sincronia com o histórico acontece sozinha.
 */
(function () {
  'use strict';

  function modaisVisiveis() {
    var nodes = document.querySelectorAll('.modal');
    var abertos = [];
    for (var i = 0; i < nodes.length; i++) {
      if (!nodes[i].hasAttribute('hidden')) {
        abertos.push(nodes[i]);
      }
    }
    return abertos;
  }

  var estadoEmpurrado = false;
  var agendado = false;

  function sincronizar() {
    var temAberto = modaisVisiveis().length > 0;
    if (temAberto && !estadoEmpurrado) {
      // Modal abriu: cria um ponto no histórico para o Voltar "consumir".
      estadoEmpurrado = true;
      history.pushState({ modalSuperCopo: true }, '');
    } else if (!temAberto && estadoEmpurrado) {
      // Modal fechado pela interface: desfaz o estado extra (sem sair da página).
      estadoEmpurrado = false;
      if (history.state && history.state.modalSuperCopo) {
        history.back();
      }
    }
  }

  function agendarSincronizar() {
    if (agendado) {
      return;
    }
    agendado = true;
    // Aguarda transições síncronas (ex.: fecha "palpite" e abre "sucesso")
    // assentarem antes de mexer no histórico.
    setTimeout(function () {
      agendado = false;
      sincronizar();
    }, 0);
  }

  function aoVoltar() {
    var abertos = modaisVisiveis();
    if (abertos.length) {
      // O navegador já removeu nosso estado; aqui só fechamos os modais.
      estadoEmpurrado = false;
      for (var i = 0; i < abertos.length; i++) {
        abertos[i].setAttribute('hidden', '');
      }
    }
  }

  document.addEventListener('DOMContentLoaded', function () {
    var modais = document.querySelectorAll('.modal');
    if (!modais.length || typeof MutationObserver === 'undefined') {
      return;
    }

    var observador = new MutationObserver(agendarSincronizar);
    for (var i = 0; i < modais.length; i++) {
      observador.observe(modais[i], {
        attributes: true,
        attributeFilter: ['hidden'],
      });
    }

    window.addEventListener('popstate', aoVoltar);

    // Caso algum modal já esteja aberto no carregamento.
    sincronizar();
  });
})();
