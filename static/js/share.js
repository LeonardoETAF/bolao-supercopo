/**
 * share.js
 * Funções de compartilhamento do Bolão Super Copo da Copa.
 *
 * Expõe globalmente:
 *  - compartilharWhatsapp()
 *  - compartilharInstagram()
 */
(function () {
  'use strict';

  // Mensagem padrão de convite usada nos dois canais.
  var MENSAGEM_CONVITE =
    '⚽ Participe do Bolão Super Copo da Copa! ' +
    'Dê seu palpite, concorra a prêmios e ganhe cupom de desconto. 🏆';

  /**
   * Abre o WhatsApp (web ou app) com a mensagem de convite + URL atual.
   */
  function compartilharWhatsapp() {
    var url = window.location.href;
    var texto = MENSAGEM_CONVITE + '\n' + url;
    var link = 'https://wa.me/?text=' + encodeURIComponent(texto);
    window.open(link, '_blank');
  }

  /**
   * Instagram não possui deep-link de compartilhamento via web,
   * então copiamos uma legenda pronta para a área de transferência
   * e orientamos o usuário a colar no story/post.
   */
  function compartilharInstagram() {
    var url = window.location.href;
    var legenda =
      MENSAGEM_CONVITE +
      '\n\n👉 ' + url +
      '\n\n#BolaoSuperCopo #Copa #Futebol';

    // Tenta usar a Clipboard API moderna.
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(legenda).then(
        function () {
          alert('Legenda copiada! Cole no seu story 📸');
        },
        function () {
          // Falha ao copiar: abre o Instagram como alternativa.
          abrirInstagram();
        }
      );
    } else {
      // Sem suporte a clipboard: tenta fallback antigo e abre o Instagram.
      var copiado = copiarFallback(legenda);
      if (copiado) {
        alert('Legenda copiada! Cole no seu story 📸');
      } else {
        abrirInstagram();
      }
    }
  }

  /**
   * Abre o site do Instagram em nova aba.
   */
  function abrirInstagram() {
    window.open('https://instagram.com', '_blank');
  }

  /**
   * Fallback de cópia para navegadores sem Clipboard API.
   * Retorna true se conseguiu copiar.
   */
  function copiarFallback(texto) {
    try {
      var area = document.createElement('textarea');
      area.value = texto;
      area.setAttribute('readonly', '');
      area.style.position = 'absolute';
      area.style.left = '-9999px';
      document.body.appendChild(area);
      area.select();
      var ok = document.execCommand('copy');
      document.body.removeChild(area);
      return ok;
    } catch (e) {
      return false;
    }
  }

  // Expõe no escopo global (chamadas a partir de onclick no HTML).
  window.compartilharWhatsapp = compartilharWhatsapp;
  window.compartilharInstagram = compartilharInstagram;
})();
