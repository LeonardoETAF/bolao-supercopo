// Painel administrativo — login JWT + gestão de jogos, resultados, métricas e cupons.
(() => {
  "use strict";

  // Acesso seguro ao localStorage: alguns navegadores/origins (ex.: 0.0.0.0,
  // modo privado) bloqueiam storage e lançam exceção — o que derrubaria o
  // script inteiro. Com try/catch, o login funciona mesmo sem persistir sessão.
  const store = {
    get(k) { try { return localStorage.getItem(k); } catch (_) { return null; } },
    set(k, v) { try { localStorage.setItem(k, v); } catch (_) {} },
    del(k) { try { localStorage.removeItem(k); } catch (_) {} },
  };

  let token = store.get("admin_token") || null;
  let userRole = store.get("admin_role") || "admin";
  let jogosCache = [];
  let landingJogoId = null; // jogo vinculado ao confronto da config
  let bandeirasCache = []; // bandeiras disponíveis em static/img
  let eventSource = null; // SSE: LEADS/métricas em tempo real
  const REDES = ["instagram", "facebook", "tiktok", "whatsapp", "youtube"];

  const $ = (sel) => document.querySelector(sel);
  const authHeaders = () => ({
    "Content-Type": "application/json",
    Authorization: `Bearer ${token}`,
  });

  function escapeHtml(s) {
    return String(s ?? "").replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
  }

  function mostrarPainel() {
    const login = $("#admin-login");
    const painel = $("#admin-painel");
    const logout = $("#btn-logout");
    if (login) login.hidden = true;
    if (painel) painel.hidden = false;
    if (logout) logout.hidden = false;
    aplicarPapel();
    carregarTudo();
    iniciarSSE();
  }

  // Viewer (somente leitura): mostra apenas Métricas e Leads; admin vê tudo.
  function aplicarPapel() {
    const viewer = userRole === "viewer";
    document.body.classList.toggle("is-viewer", viewer);
    // Cards exclusivos do acesso total: Jogos e Configuração da Landing.
    const cardJogos = $("#card-jogos");
    const landing = $("#landing-config");
    if (cardJogos) cardJogos.hidden = viewer;
    if (landing) landing.hidden = viewer;
    // O card de Redes Sociais (último admin__card) também é só do admin.
    const redesCard = document.querySelector('[data-rede-ativo="instagram"]');
    const cardRedes = redesCard ? redesCard.closest(".admin__card") : null;
    if (cardRedes) cardRedes.hidden = viewer;
  }

  // ---- Login ----
  async function login(e) {
    e.preventDefault();
    const form = e.target;
    const erro = $("#erro-login");
    if (erro) erro.textContent = "";
    const payload = {
      usuario: form.usuario.value.trim(),
      senha: form.senha.value,
    };
    try {
      const res = await fetch("/admin/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.erro || "Falha no login");
      token = data.token;
      userRole = data.role || "admin";
      store.set("admin_token", token);
      store.set("admin_role", userRole);
      mostrarPainel();
    } catch (err) {
      if (erro) erro.textContent = err.message;
    }
  }

  // ---- Carregamentos ----
  async function carregarTudo() {
    // Viewer só enxerga Métricas, Classificação Final e Leads.
    if (userRole === "viewer") {
      await Promise.all([
        carregarMetricas(),
        carregarClassificacaoFinal(),
        carregarCupons(),
      ]);
      return;
    }
    await Promise.all([
      carregarMetricas(),
      carregarClassificacaoFinal(),
      carregarJogos(),
      carregarCupons(),
      carregarLanding(),
      carregarBandeiras(),
    ]);
  }

  // ---- Bandeiras disponíveis (para o cadastro de jogos) ----
  async function carregarBandeiras() {
    const res = await fetch("/admin/bandeiras", { headers: authHeaders() });
    if (res.status === 401) return sair();
    bandeirasCache = await res.json();
    preencherSelectsBandeira();
  }

  function preencherSelectsBandeira() {
    document.querySelectorAll("select[data-flagprev]").forEach((sel) => {
      const atual = sel.value;
      sel.innerHTML = bandeirasCache
        .map((b) => `<option value="${b.url}">${escapeHtml(b.nome)}</option>`)
        .join("");
      if (atual) sel.value = atual;
      atualizarPreviewBandeira(sel);
    });
  }

  function atualizarPreviewBandeira(sel) {
    const prev = document.getElementById(sel.dataset.flagprev);
    if (prev && sel.value) prev.innerHTML = `<img src="${sel.value}" alt="">`;
  }

  // ======================= CONFIGURAÇÃO DA LANDING =======================
  // Mapeia o nome do time para a bandeira disponível (espelha bandeira() do servidor).
  function bandeiraDe(nome) {
    switch (String(nome || "").trim().toLowerCase()) {
      case "brasil":
      case "brazil":
        return "/static/img/Flag_of_Brazil.svg";
      case "marrocos":
      case "morocco":
      case "maroc":
        return "/static/img/Flag_of_Morocco.svg";
      case "haiti":
      case "haíti":
      case "haïti":
        return "/static/img/Flag_of_Haiti.svg";
      case "escócia":
      case "escocia":
      case "scotland":
        return "/static/img/Flag_of_Scotland.svg";
      default:
        return null;
    }
  }

  function setFlag(id, url, alt) {
    const el = document.getElementById(id);
    if (!el) return;
    if (url) {
      el.innerHTML = `<img src="${url}" alt="${escapeHtml(alt)}">`;
      el.classList.remove("cfg-flag--ph");
    } else {
      el.textContent = "⚽";
      el.classList.add("cfg-flag--ph");
    }
  }

  // Converte o ISO (UTC) do jogo para data/horário no fuso de Brasília.
  function isoParaDataHora(iso) {
    const d = new Date(iso);
    const p = new Intl.DateTimeFormat("pt-BR", {
      timeZone: "America/Sao_Paulo",
      day: "2-digit", month: "2-digit", year: "numeric",
      hour: "2-digit", minute: "2-digit", hour12: false,
    })
      .formatToParts(d)
      .reduce((a, x) => ((a[x.type] = x.value), a), {});
    return { data: `${p.day}/${p.month}/${p.year}`, horario: `${p.hour}h${p.minute}` };
  }

  // Preenche o <select> "Jogo atual" apenas com os jogos cadastrados (o cadastro
  // é feito no card Jogos; aqui só se seleciona qual fica ativo no site).
  function popularSelectJogos() {
    const sel = $("#jogo-atual");
    if (!sel) return;
    const opts = jogosCache.length
      ? ['<option value="" disabled>Selecione o jogo atual…</option>']
      : ['<option value="">Nenhum jogo cadastrado</option>'];
    jogosCache.forEach((j) => {
      const { data, horario } = isoParaDataHora(j.data_jogo);
      const marca = j.status === "encerrado" ? " (encerrado)" : j.ativo ? " (ativo)" : "";
      opts.push(
        `<option value="${j.id}">${escapeHtml(j.time_a)} x ${escapeHtml(j.time_b)} — ${data} ${horario}${marca}</option>`
      );
    });
    sel.innerHTML = opts.join("");
    sel.value = landingJogoId || "";
    atualizarConfrontoDoJogo();
  }

  // Mostra (somente leitura) os dados do jogo atual selecionado no confronto.
  function atualizarConfrontoDoJogo() {
    const box = $("#landing-config");
    if (!box) return;
    const set = (name, v) => {
      const el = box.querySelector(`[name="${name}"]`);
      if (el) el.value = v;
    };
    const j = jogosCache.find((x) => x.id === landingJogoId);
    if (j) {
      set("time1_nome", j.time_a);
      set("time2_nome", j.time_b);
      const { data, horario } = isoParaDataHora(j.data_jogo);
      set("data", data);
      set("horario", horario);
      const fa = j.bandeira_a || bandeiraDe(j.time_a);
      const fb = j.bandeira_b || bandeiraDe(j.time_b);
      setFlag("flag-time1", fa, j.time_a);
      setFlag("flag-res1", fa, j.time_a);
      setFlag("flag-time2", fb, j.time_b);
      setFlag("flag-res2", fb, j.time_b);
    } else {
      set("time1_nome", "");
      set("time2_nome", "");
      set("data", "");
      set("horario", "");
    }
    atualizarResultado();
  }

  // Troca do jogo atual no <select> do confronto.
  function selecionarJogo() {
    landingJogoId = $("#jogo-atual").value || null;
    atualizarConfrontoDoJogo();
  }

  async function carregarLanding() {
    const box = $("#landing-config");
    if (!box) return;
    const res = await fetch("/admin/landing", { headers: authHeaders() });
    if (res.status === 401) return sair();
    const cfg = await res.json();
    landingJogoId = cfg.jogo_id || null;

    const set = (name, valor) => {
      const el = box.querySelector(`[name="${name}"]`);
      if (el) el.value = valor ?? "";
    };
    // Confronto (times/data/horário) e placar vêm do jogo atual, não do config.
    set("cupom_participacao_desconto", cfg.cupom_participacao_desconto);
    set("cupom_acerto_desconto", cfg.cupom_acerto_desconto);

    REDES.forEach((rede) => {
      const r = cfg[rede] || { ativo: false, url: "" };
      const at = document.querySelector(`[data-rede-ativo="${rede}"]`);
      const url = document.querySelector(`[data-rede-url="${rede}"]`);
      if (at) at.checked = !!r.ativo;
      if (url) url.value = r.url || "";
    });

    // popularSelectJogos() → atualizarConfrontoDoJogo() preenche os campos do jogo.
    popularSelectJogos();
  }

  // Reflete o placar/status do jogo atual no card Resultado Oficial.
  function atualizarResultado() {
    const box = $("#landing-config");
    if (!box) return;
    const j = jogosCache.find((x) => x.id === landingJogoId);
    const set = (n, v) => {
      const e = box.querySelector(`[name="${n}"]`);
      if (e) e.value = v;
    };
    const st = $("#status-resultado");
    if (j) {
      set("placar_time1", j.placar_a != null ? j.placar_a : "");
      set("placar_time2", j.placar_b != null ? j.placar_b : "");
      if (st)
        st.textContent =
          j.status === "encerrado"
            ? `✓ Resultado divulgado: ${j.time_a} ${j.placar_a} x ${j.placar_b} ${j.time_b}.`
            : "Resultado ainda não divulgado.";
    } else if (st) {
      st.textContent = "Selecione o jogo atual no Confronto para informar o resultado.";
    }
  }

  // Divulga o resultado do jogo atual: encerra, apura o ranking e gera cupons 30%.
  async function divulgarResultado() {
    const box = $("#landing-config");
    const btn = $("#btn-divulgar-resultado");
    const st = $("#status-resultado");
    if (!landingJogoId) {
      if (st) st.textContent = "Selecione o jogo atual no Confronto antes de divulgar.";
      return;
    }
    const a = Number(box.querySelector('[name="placar_time1"]').value) || 0;
    const b = Number(box.querySelector('[name="placar_time2"]').value) || 0;
    if (btn) btn.disabled = true;
    if (st) st.textContent = "Divulgando resultado…";
    try {
      const res = await fetch(`/admin/jogos/${landingJogoId}/resultado`, {
        method: "PUT",
        headers: authHeaders(),
        body: JSON.stringify({ gols_time_a: a, gols_time_b: b }),
      });
      if (res.status === 401) return sair();
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.erro || "Erro ao divulgar resultado");
      if (st)
        st.textContent = `Resultado divulgado! ${data.processados} palpites apurados, ${data.cupons_30} cupons gerados.`;
      await carregarJogos();
      await carregarMetricas();
      await carregarClassificacaoFinal();
      atualizarResultado();
    } catch (err) {
      if (st) st.textContent = err.message;
    } finally {
      if (btn) btn.disabled = false;
    }
  }

  async function salvarLanding() {
    const box = $("#landing-config");
    if (!box) return;
    const erro = $("#erro-salvar-landing");
    const btn = $("#btn-salvar-landing");
    if (erro) erro.textContent = "";
    const val = (name) => box.querySelector(`[name="${name}"]`).value.trim();

    const payload = {
      jogo_id: landingJogoId,
      time1_nome: val("time1_nome"),
      time2_nome: val("time2_nome"),
      data: val("data"),
      horario: val("horario"),
      placar_time1: Number(val("placar_time1")) || 0,
      placar_time2: Number(val("placar_time2")) || 0,
      cupom_participacao_desconto: Number(val("cupom_participacao_desconto")) || 0,
      cupom_acerto_desconto: Number(val("cupom_acerto_desconto")) || 0,
    };
    REDES.forEach((rede) => {
      const ativoEl = document.querySelector(`[data-rede-ativo="${rede}"]`);
      const urlEl = document.querySelector(`[data-rede-url="${rede}"]`);
      const url = urlEl ? urlEl.value.trim() : "";
      // Sem URL não há link para exibir: a rede não pode ficar "ativa".
      const ativo = ativoEl ? ativoEl.checked && url !== "" : false;
      if (ativoEl && !url) ativoEl.checked = false;
      payload[rede] = { ativo, url };
    });

    const txt = btn ? btn.textContent : "";
    if (btn) {
      btn.disabled = true;
      btn.textContent = "Salvando...";
    }
    try {
      const res = await fetch("/admin/landing", {
        method: "PUT",
        headers: authHeaders(),
        body: JSON.stringify(payload),
      });
      if (res.status === 401) return sair();
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.erro || "Erro ao salvar");
      // Mantém o jogo selecionado e reflete na lista de jogos.
      landingJogoId = data.jogo_id || landingJogoId;
      carregarJogos();
      carregarMetricas();
      if (btn) {
        btn.textContent = "Salvo ✓";
        setTimeout(() => (btn.textContent = txt), 1600);
      }
    } catch (err) {
      if (erro) erro.textContent = err.message;
      if (btn) btn.textContent = txt;
    } finally {
      if (btn) btn.disabled = false;
    }
  }

  async function carregarMetricas() {
    const alvo = $("#metricas");
    if (!alvo) return;
    const res = await fetch("/admin/metricas", { headers: authHeaders() });
    if (res.status === 401) return sair();
    const m = await res.json();
    const card = (valor, label) =>
      `<div class="metric-card"><span class="metric-card__valor">${escapeHtml(valor)}</span><span class="metric-card__label">${label}</span></div>`;
    alvo.innerHTML =
      card(m.total_participantes, "Participantes") +
      card(m.total_palpites, "Palpites") +
      card(m.cupons_gerados, "Cupons gerados") +
      card(m.cupons_utilizados, "Cupons usados") +
      card((m.taxa_acerto ?? 0) + "%", "Taxa de acerto");
  }

  async function carregarJogos() {
    const alvo = $("#lista-jogos");
    if (!alvo) return;
    const res = await fetch("/admin/jogos", { headers: authHeaders() });
    if (res.status === 401) return sair();
    const jogos = await res.json();
    jogosCache = jogos;
    popularSelectJogos();
    atualizarResultado();
    if (!jogos.length) {
      alvo.innerHTML = "<p>Nenhum jogo cadastrado.</p>";
      return;
    }
    alvo.innerHTML = jogos
      .map((j) => {
        const { data, horario } = isoParaDataHora(j.data_jogo);
        const placar = j.placar_a != null ? ` · ${j.placar_a} x ${j.placar_b}` : "";
        const toggle =
          j.status === "encerrado"
            ? ""
            : j.ativo
            ? `<button class="btn btn--secundario" data-desativar="${j.id}">Desativar</button>`
            : `<button class="btn btn--secundario" data-ativar="${j.id}">Ativar</button>`;
        return `
          <div class="admin-jogo" data-jogo-row="${j.id}">
            <div class="admin-jogo__info">
              ${flagJogo(j, "a")}
              <strong>${escapeHtml(j.time_a)} x ${escapeHtml(j.time_b)}</strong>
              ${flagJogo(j, "b")}
              <span class="admin-jogo__data">${data} ${horario}</span>
              <span class="status-${j.status}">${j.status}${placar}</span>
            </div>
            <div class="admin-jogo__acoes">
              ${toggle}
              <button class="btn btn--secundario" data-editar="${j.id}">Editar</button>
              <button class="btn btn--excluir" data-excluir="${j.id}">Excluir</button>
            </div>
          </div>`;
      })
      .join("");

    alvo.querySelectorAll("[data-ativar]").forEach((b) =>
      b.addEventListener("click", () => ativarJogo(b.dataset.ativar))
    );
    alvo.querySelectorAll("[data-desativar]").forEach((b) =>
      b.addEventListener("click", () => desativarJogo(b.dataset.desativar))
    );
    alvo.querySelectorAll("[data-excluir]").forEach((b) =>
      b.addEventListener("click", () => excluirJogo(b.dataset.excluir))
    );
    alvo.querySelectorAll("[data-editar]").forEach((b) =>
      b.addEventListener("click", () => editarJogo(b.dataset.editar))
    );
  }

  // <span> com a bandeira do time (escolhida no cadastro ou mapeada pelo nome).
  function flagJogo(j, lado) {
    const nome = lado === "a" ? j.time_a : j.time_b;
    const url = (lado === "a" ? j.bandeira_a : j.bandeira_b) || bandeiraDe(nome);
    return url
      ? `<span class="admin-jogo__flag"><img src="${url}" alt=""></span>`
      : `<span class="admin-jogo__flag admin-jogo__flag--ph">⚽</span>`;
  }

  // Converte o ISO (UTC) para o formato de <input type="datetime-local"> em Brasília.
  function isoParaInputLocal(iso) {
    const p = new Intl.DateTimeFormat("pt-BR", {
      timeZone: "America/Sao_Paulo",
      day: "2-digit", month: "2-digit", year: "numeric",
      hour: "2-digit", minute: "2-digit", hour12: false,
    })
      .formatToParts(new Date(iso))
      .reduce((a, x) => ((a[x.type] = x.value), a), {});
    return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}`;
  }

  // Edição inline da linha do jogo (sem diálogos do navegador).
  function editarJogo(id) {
    const j = jogosCache.find((x) => x.id === id);
    const row = document.querySelector(`[data-jogo-row="${id}"]`);
    if (!j || !row) return;
    row.classList.add("admin-jogo--edit");
    row.innerHTML = `
      <div class="admin-jogo__edit">
        <input type="text" data-edit-a value="${escapeHtml(j.time_a)}" placeholder="Time A">
        <span class="admin-jogo__x">x</span>
        <input type="text" data-edit-b value="${escapeHtml(j.time_b)}" placeholder="Time B">
        <input type="date" data-edit-data value="${isoParaInputLocal(j.data_jogo).split("T")[0]}">
        <input type="time" data-edit-hora value="${isoParaInputLocal(j.data_jogo).split("T")[1]}">
      </div>
      <div class="admin-jogo__acoes">
        <button class="btn btn--primario" data-salvar-edit>Salvar</button>
        <button class="btn btn--secundario" data-cancelar-edit>Cancelar</button>
      </div>
      <span class="form__erro" data-edit-erro></span>`;

    row.querySelector("[data-cancelar-edit]").addEventListener("click", carregarJogos);
    row.querySelector("[data-salvar-edit]").addEventListener("click", async () => {
      const erro = row.querySelector("[data-edit-erro]");
      if (erro) erro.textContent = "";
      const ta = row.querySelector("[data-edit-a]").value.trim();
      const tb = row.querySelector("[data-edit-b]").value.trim();
      const dataVal = row.querySelector("[data-edit-data]").value;
      const horaVal = row.querySelector("[data-edit-hora]").value;
      if (!ta || !tb || !dataVal || !horaVal) {
        if (erro) erro.textContent = "Preencha os times, a data e o horário.";
        return;
      }
      const res = await fetch(`/admin/jogos/${id}`, {
        method: "PUT",
        headers: authHeaders(),
        body: JSON.stringify({
          time_a: ta,
          time_b: tb,
          data_jogo: new Date(`${dataVal}T${horaVal}`).toISOString(),
        }),
      });
      if (res.status === 401) return sair();
      if (res.ok) carregarTudo();
      else {
        const d = await res.json().catch(() => ({}));
        if (erro) erro.textContent = d.erro || "Erro ao editar jogo";
      }
    });
  }

  let cuponsCache = [];
  let filtroCupons = "";
  let buscaCupons = "";

  async function carregarClassificacaoFinal() {
    const alvo = $("#lista-classificacao");
    if (!alvo) return;
    const res = await fetch("/admin/classificacao-final", {
      headers: authHeaders(),
    });
    if (res.status === 401) return sair();
    const linhas = await res.json();
    if (!Array.isArray(linhas) || linhas.length === 0) {
      alvo.innerHTML =
        '<p class="classif-vazia">Ainda não há pontuações para classificar.</p>';
      return;
    }
    // Campeão = elegível com a maior pontuação. Empate na maior pontuação
    // gera co-campeões (o backend marca todos com r.campeao).
    alvo.innerHTML = linhas
      .map((r) => {
        const ehCampeao = !!r.campeao;
        const faltam = Math.max(0, r.cupons_total - r.cupons_utilizados);
        const status = r.elegivel
          ? '<span class="classif-badge classif-badge--ok">Elegível</span>'
          : `<span class="classif-badge classif-badge--no">Faltam ${faltam} cupom${faltam === 1 ? "" : "s"}</span>`;
        const selo = ehCampeao
          ? ' <span class="classif-badge classif-badge--camp">🏆 Campeão</span>'
          : "";
        return (
          `<div class="classif-row${ehCampeao ? " is-campeao" : ""}${r.elegivel ? "" : " is-inelegivel"}">` +
          `<span class="classif-pos">${r.posicao}º</span>` +
          `<div class="classif-info">` +
          `<span class="classif-nome">${escapeHtml(r.nome)}${selo}</span>` +
          `<span class="classif-sub">${r.total_pontos} pts · ${r.acertos_exatos} cravada${r.acertos_exatos === 1 ? "" : "s"} · cupons ${r.cupons_utilizados}/${r.cupons_total}</span>` +
          `</div>` +
          status +
          `</div>`
        );
      })
      .join("");
  }

  async function carregarCupons() {
    const alvo = $("#lista-cupons");
    if (!alvo) return;
    const url = filtroCupons ? `/admin/cupons?filtro=${filtroCupons}` : "/admin/cupons";
    const res = await fetch(url, { headers: authHeaders() });
    if (res.status === 401) return sair();
    cuponsCache = await res.json();
    renderizarCupons();
  }

  function renderizarCupons() {
    const alvo = $("#lista-cupons");
    if (!alvo) return;
    const termo = buscaCupons.trim().toLowerCase();
    const lista = termo
      ? cuponsCache.filter(
          (c) =>
            c.codigo.toLowerCase().includes(termo) ||
            String(c.tipo).toLowerCase().includes(termo)
        )
      : cuponsCache;

    if (!lista.length) {
      alvo.innerHTML = `<p class="cupons-vazio">${
        termo ? "Nenhum cupom encontrado para a busca." : "Nenhum cupom."
      }</p>`;
      return;
    }

    const podeBaixar = userRole !== "viewer";
    alvo.innerHTML =
      `<table><thead><tr><th>Código</th><th>Tipo</th><th>Status</th>${
        podeBaixar ? "<th>Ação</th>" : ""
      }</tr></thead><tbody>` +
      lista
        .map(
          (c) =>
            `<tr>` +
            `<td class="cupom-codigo">${escapeHtml(c.codigo)}</td>` +
            `<td class="cupom-tipo">${escapeHtml(c.tipo)}</td>` +
            `<td><span class="cupom-status ${
              c.utilizado ? "cupom-status--uso" : "cupom-status--disp"
            }">${c.utilizado ? "Utilizado" : "Disponível"}</span></td>` +
            (podeBaixar
              ? `<td>${
                  c.utilizado
                    ? ""
                    : `<button class="btn-baixa" data-usar="${c.id}">Dar baixa</button>`
                }</td>`
              : "") +
            `</tr>`
        )
        .join("") +
      `</tbody></table>`;

    alvo.querySelectorAll("[data-usar]").forEach((b) =>
      b.addEventListener("click", () => marcarCupom(b.dataset.usar))
    );
  }

  async function marcarCupom(id) {
    const res = await fetch(`/admin/cupons/${id}/utilizar`, {
      method: "PUT",
      headers: authHeaders(),
    });
    if (res.status === 401) return sair();
    if (res.ok) {
      carregarCupons();
      carregarMetricas();
      // Dar baixa num cupom pode alterar a elegibilidade ao prêmio final.
      carregarClassificacaoFinal();
    } else {
      console.error("Erro ao dar baixa no cupom");
    }
  }

  async function excluirJogo(id) {
    const erro = $("#erro-jogo");
    if (erro) erro.textContent = "";
    const res = await fetch(`/admin/jogos/${id}`, {
      method: "DELETE",
      headers: authHeaders(),
    });
    if (res.status === 401) return sair();
    if (res.ok || res.status === 204) {
      carregarTudo();
    } else {
      const d = await res.json().catch(() => ({}));
      if (erro) erro.textContent = d.erro || "Erro ao excluir jogo";
    }
  }

  // ---- Cadastro e ativação de jogos ----
  async function cadastrarJogo(e) {
    e.preventDefault();
    const form = e.target;
    const erro = $("#erro-jogo");
    if (erro) erro.textContent = "";
    if (!form.data.value || !form.hora.value) {
      if (erro) erro.textContent = "Informe a data e o horário do jogo.";
      return;
    }
    const payload = {
      time_a: form.time_a.value.trim(),
      time_b: form.time_b.value.trim(),
      data_jogo: new Date(`${form.data.value}T${form.hora.value}`).toISOString(),
      bandeira_a: form.bandeira_a.value || null,
      bandeira_b: form.bandeira_b.value || null,
    };
    try {
      const res = await fetch("/admin/jogos", {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(payload),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.erro || "Falha ao cadastrar");
      form.reset();
      preencherSelectsBandeira();
      await carregarJogos();
      await carregarMetricas();
    } catch (err) {
      if (erro) erro.textContent = err.message;
    }
  }

  async function ativarJogo(id) {
    const res = await fetch(`/admin/jogos/${id}/ativar`, {
      method: "PUT",
      headers: authHeaders(),
    });
    if (res.ok) carregarJogos();
  }

  async function desativarJogo(id) {
    const res = await fetch(`/admin/jogos/${id}/desativar`, {
      method: "PUT",
      headers: authHeaders(),
    });
    if (res.ok) carregarJogos();
  }

  // SSE: mantém LEADS e métricas atualizados em tempo real (igual ao ranking).
  function iniciarSSE() {
    if (typeof EventSource === "undefined" || eventSource) return;
    try {
      eventSource = new EventSource("/api/ranking/stream");
      eventSource.onmessage = () => {
        carregarMetricas();
      };
      eventSource.onerror = () => {}; // o EventSource reconecta sozinho
    } catch (e) {
      console.error("SSE do admin falhou:", e);
    }
  }

  function pararSSE() {
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }
  }

  function sair() {
    pararSSE();
    token = null;
    userRole = "admin";
    store.del("admin_token");
    store.del("admin_role");
    const login = $("#admin-login");
    const painel = $("#admin-painel");
    const logout = $("#btn-logout");
    if (login) login.hidden = false;
    if (painel) painel.hidden = true;
    if (logout) logout.hidden = true;
  }

  document.addEventListener("DOMContentLoaded", () => {
    const formLogin = $("#form-login");
    if (formLogin) formLogin.addEventListener("submit", login);
    const btnLogout = $("#btn-logout");
    if (btnLogout) btnLogout.addEventListener("click", sair);
    const filtro = $("#filtro-cupons");
    if (filtro) {
      filtro.addEventListener("click", (ev) => {
        const chip = ev.target.closest(".cupom-chip");
        if (!chip) return;
        filtro.querySelectorAll(".cupom-chip").forEach((c) => c.classList.remove("is-ativo"));
        chip.classList.add("is-ativo");
        filtroCupons = chip.dataset.filtro || "";
        carregarCupons();
      });
    }
    const buscaCp = $("#busca-cupons");
    if (buscaCp) {
      buscaCp.addEventListener("input", () => {
        buscaCupons = buscaCp.value;
        renderizarCupons();
      });
    }
    const btnLanding = $("#btn-salvar-landing");
    if (btnLanding) btnLanding.addEventListener("click", salvarLanding);
    const btnDivulgar = $("#btn-divulgar-resultado");
    if (btnDivulgar) btnDivulgar.addEventListener("click", divulgarResultado);
    const selJogo = $("#jogo-atual");
    if (selJogo) selJogo.addEventListener("change", selecionarJogo);
    const formJogo = $("#form-jogo");
    if (formJogo) formJogo.addEventListener("submit", cadastrarJogo);
    const btnReloadLeads = $("#btn-recarregar-leads");
    if (btnReloadLeads)
      btnReloadLeads.addEventListener("click", () => {
        carregarCupons();
        carregarMetricas();
      });
    const btnReloadClassif = $("#btn-recarregar-classif");
    if (btnReloadClassif)
      btnReloadClassif.addEventListener("click", carregarClassificacaoFinal);
    document
      .querySelectorAll("select[data-flagprev]")
      .forEach((sel) =>
        sel.addEventListener("change", () => atualizarPreviewBandeira(sel))
      );
    // Valida o token salvo antes de exibir o painel: evita ficar preso com
    // uma sessão antiga/inválida (ex.: token sem o papel após a atualização).
    if (token) {
      fetch("/admin/metricas", { headers: authHeaders() })
        .then((r) => (r.ok ? mostrarPainel() : sair()))
        .catch(() => sair());
    }
  });
})();
