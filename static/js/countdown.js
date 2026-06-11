/**
 * countdown.js
 * Contagem regressiva até o início do jogo.
 *
 * Lê o elemento #countdown[data-target] (data ISO 8601) e atualiza
 * os spans #cd-h / #cd-m / #cd-s a cada segundo.
 * Ao zerar, substitui o conteúdo por "⚽ Jogo em andamento!".
 */
document.addEventListener('DOMContentLoaded', function () {
  var countdownEl = document.getElementById('countdown');

  // Se não houver contador nesta página, não faz nada.
  if (!countdownEl) {
    return;
  }

  var targetAttr = countdownEl.getAttribute('data-target');
  if (!targetAttr) {
    return;
  }

  // Converte a data alvo (ISO 8601) para milissegundos.
  var targetTime = new Date(targetAttr).getTime();
  if (isNaN(targetTime)) {
    // data-target inválido: não há o que contar.
    return;
  }

  var elD = document.getElementById('cd-d');
  var elH = document.getElementById('cd-h');
  var elM = document.getElementById('cd-m');
  var elS = document.getElementById('cd-s');

  /**
   * Garante 2 dígitos (ex.: 5 -> "05").
   */
  function doisDigitos(valor) {
    return String(valor).padStart(2, '0');
  }

  /**
   * Atualiza os spans com base no tempo restante.
   * Retorna false quando o tempo acabou (para encerrar o loop).
   */
  function atualizar() {
    var agora = Date.now();
    var diff = targetTime - agora;

    // Chegou a zero ou ficou negativo: jogo começou.
    if (diff <= 0) {
      if (intervalo) {
        clearInterval(intervalo);
      }
      countdownEl.textContent = '⚽ Jogo em andamento!';
      return false;
    }

    var totalSegundos = Math.floor(diff / 1000);

    // Se houver bloco de dias (#cd-d), exibe dias separados e horas de 0–23.
    // Caso contrário, acumula os dias dentro das horas.
    var horas;
    if (elD) {
      elD.textContent = doisDigitos(Math.floor(totalSegundos / 86400));
      horas = Math.floor((totalSegundos % 86400) / 3600);
    } else {
      horas = Math.floor(totalSegundos / 3600);
    }
    var minutos = Math.floor((totalSegundos % 3600) / 60);
    var segundos = totalSegundos % 60;

    if (elH) {
      elH.textContent = doisDigitos(horas);
    }
    if (elM) {
      elM.textContent = doisDigitos(minutos);
    }
    if (elS) {
      elS.textContent = doisDigitos(segundos);
    }

    return true;
  }

  // Primeira atualização imediata para não esperar 1s.
  var continuar = atualizar();

  // Só cria o intervalo se ainda houver tempo restante.
  var intervalo = null;
  if (continuar !== false) {
    intervalo = setInterval(atualizar, 1000);
  }
});
