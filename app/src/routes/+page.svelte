<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    api, OPT, ITF, LED_DRIVERS, LED_ORDERS,
    type Phy, type DeviceInfo, type FidoInfo, type Passkey,
  } from "$lib/api";

  type Tab = "device" | "led" | "button" | "fido" | "advanced";
  const tabs: { id: Tab; title: string; icon: string }[] = [
    { id: "device", title: "Устройство", icon: "🔑" },
    { id: "led", title: "Индикация", icon: "💡" },
    { id: "button", title: "Кнопка", icon: "👆" },
    { id: "fido", title: "FIDO и PIN", icon: "🛡️" },
    { id: "advanced", title: "Дополнительно", icon: "⚙️" },
  ];
  let tab = $state<Tab>("device");

  let admin = $state(false);
  let info = $state<DeviceInfo | null>(null);
  let fw = $state("");
  let phy = $state<Phy | null>(null);
  let connected = $state(false);

  let busy = $state(false);
  let waitButton = $state(false);
  let toast = $state<{ kind: "ok" | "err" | "info"; text: string } | null>(null);

  function show(kind: "ok" | "err" | "info", text: string) {
    toast = { kind, text };
    if (kind !== "err") setTimeout(() => (toast = null), 5000);
  }

  async function run<T>(fn: () => Promise<T>, okText?: string): Promise<T | undefined> {
    busy = true;
    try {
      const r = await fn();
      if (okText) show("ok", okText);
      return r;
    } catch (e) {
      show("err", String(e));
    } finally {
      busy = false;
    }
  }

  async function refresh() {
    busy = true;
    try {
      const st = await api.appStatus();
      admin = st.admin;
      info = await api.deviceInfo();
      fw = await api.firmwareVersion().catch(() => "");
      phy = await api.readPhy();
      connected = true;
      toast = null;
    } catch (e) {
      connected = false;
      info = null;
      phy = null;
      show("err", String(e));
    } finally {
      busy = false;
    }
  }

  onMount(refresh);

  // ---------- PHY ----------
  const buttonOn = $derived(!!phy?.up_btn);

  async function savePhy(p: Phy, text = "Настройки записаны. Отключите и подключите ключ, чтобы применить их.") {
    waitButton = buttonOn;
    await run(() => api.writePhy($state.snapshot(p) as Phy), text);
    waitButton = false;
    await refresh();
  }

  function hasOpt(o: number) {
    return !!phy && (phy.opts & o) !== 0;
  }
  function setOpt(o: number, on: boolean) {
    if (phy) phy.opts = on ? phy.opts | o : phy.opts & ~o;
  }

  // Button
  let btnEnabled = $state(false);
  let btnTimeout = $state(15);
  $effect(() => {
    if (phy) {
      btnEnabled = !!phy.up_btn;
      btnTimeout = phy.up_btn || 15;
    }
  });
  async function saveButton() {
    if (!phy) return;
    const p = $state.snapshot(phy) as Phy;
    p.up_btn = btnEnabled ? Math.max(1, Math.min(255, btnTimeout)) : 0;
    await savePhy(p);
  }

  // USB interfaces
  const itf = $derived(phy?.enabled_usb_itf ?? ITF.ALL_DEFAULT);
  function setKb(on: boolean) {
    if (phy) phy.enabled_usb_itf = on ? itf | ITF.KB : itf & ~ITF.KB;
  }

  // ---------- Device ----------
  async function rebootNormal() {
    await run(() => api.reboot(false), "Ключ перезагружается.");
    setTimeout(refresh, 3000);
  }
  async function rebootBootsel() {
    await run(() => api.reboot(true), "Ключ в режиме прошивки. Компьютер показывает диск RPI-RP2.");
    connected = false;
  }
  async function flash() {
    const path = await open({ multiple: false, filters: [{ name: "Прошивка UF2", extensions: ["uf2"] }] });
    if (!path || Array.isArray(path)) return;
    const r = await run(() => api.flashFirmware(path as string));
    if (r) show("ok", r);
    setTimeout(refresh, 6000);
  }

  // ---------- FIDO ----------
  let fido = $state<FidoInfo | null>(null);
  let pinCur = $state("");
  let pinNew = $state("");
  let pinNew2 = $state("");
  let pinList = $state("");
  let passkeys = $state<Passkey[] | null>(null);

  async function loadFido() {
    fido = (await run(() => api.fidoInfo())) ?? null;
  }
  async function savePin() {
    if (pinNew.length < (fido?.min_pin_length || 4)) {
      return show("err", `PIN слишком короткий. Минимум ${fido?.min_pin_length || 4} символа.`);
    }
    if (pinNew !== pinNew2) return show("err", "Новый PIN и подтверждение не совпадают.");
    if (fido?.pin_set) await run(() => api.changePin(pinCur, pinNew), "PIN изменён.");
    else await run(() => api.setPin(pinNew), "PIN установлен.");
    pinCur = pinNew = pinNew2 = "";
    await loadFido();
  }
  async function loadPasskeys() {
    passkeys = (await run(() => api.listPasskeys(pinList))) ?? null;
  }
  async function removePasskey(p: Passkey) {
    if (!confirm(`Удалить passkey «${p.user_name}» для ${p.rp_id}? Восстановить его нельзя.`)) return;
    await run(() => api.deletePasskey(pinList, p.credential_id), "Passkey удалён.");
    await loadPasskeys();
  }

  $effect(() => {
    if (tab === "fido" && admin && !fido) loadFido();
  });

  const fmtKb = (n: number) => `${(n / 1024).toFixed(0)} КБ`;
  const vidpid = $derived(
    phy?.vidpid
      ? `${phy.vidpid.slice(0, 2).map((b) => b.toString(16).padStart(2, "0")).join("")}:${phy.vidpid
          .slice(2)
          .map((b) => b.toString(16).padStart(2, "0"))
          .join("")}`
      : "—",
  );
</script>

<div class="app">
  <aside>
    <div class="brand">
      <div class="logo">🔑</div>
      <div>
        <div class="name">fido-key</div>
        <div class="sub">утилита настройки</div>
      </div>
    </div>
    <nav>
      {#each tabs as t}
        <button class:active={tab === t.id} onclick={() => (tab = t.id)}>
          <span>{t.icon}</span>{t.title}
        </button>
      {/each}
    </nav>
    <div class="status">
      <div class="dot" class:on={connected}></div>
      {connected ? "Ключ подключён" : "Ключ не найден"}
      <button class="ghost small" onclick={refresh} disabled={busy}>Обновить</button>
    </div>
    <div class="admin" class:on={admin}>
      {admin ? "Права администратора: есть" : "Права администратора: нет"}
    </div>
  </aside>

  <main>
    {#if waitButton}
      <div class="banner warn pulse">👆 Нажмите кнопку <b>BOOT</b> на ключе, чтобы подтвердить запись.</div>
    {/if}

    {#if tab === "device"}
      <h1>Устройство</h1>
      {#if info}
        <div class="grid">
          <div class="card stat"><span>Микроконтроллер</span><b>{info.mcu}</b></div>
          <div class="card stat"><span>Прошивка</span><b>{info.product} {fw || info.version}</b></div>
          <div class="card stat"><span>Серийный номер</span><b class="mono">{info.serial}</b></div>
          <div class="card stat"><span>USB VID:PID</span><b class="mono">{vidpid}</b></div>
        </div>
        {#if info.flash}
          <div class="card">
            <h3>Память ключа</h3>
            <div class="bar"><div style="width:{(info.flash.used / info.flash.total) * 100}%"></div></div>
            <p class="muted">
              Занято {fmtKb(info.flash.used)} из {fmtKb(info.flash.total)}. Файлов: {info.flash.files}.
              {#if info.flash.firmware}Размер прошивки: {fmtKb(info.flash.firmware)}.{/if}
            </p>
          </div>
        {/if}
      {:else}
        <div class="card"><p>Подключите ключ. Затем нажмите «Обновить».</p></div>
      {/if}
      <div class="card">
        <h3>Перезагрузка и прошивка</h3>
        <div class="row">
          <button onclick={rebootNormal} disabled={busy || !connected}>Перезагрузить</button>
          <button onclick={rebootBootsel} disabled={busy || !connected}>Режим прошивки</button>
          <button class="primary" onclick={flash} disabled={busy}>Прошить файл .uf2…</button>
        </div>
        <p class="muted">
          «Прошить файл» переводит ключ в режим прошивки и копирует файл на диск RPI-RP2.
          Данные ключа (PIN, passkeys) сохраняются, если раскладка памяти не изменилась.
        </p>
      </div>
    {/if}

    {#if tab === "led" && phy}
      <h1>Индикация</h1>
      <div class="card form">
        <label>Тип светодиода
          <select bind:value={phy.led_driver}>
            {#each LED_DRIVERS as d}<option value={d.id}>{d.name}</option>{/each}
          </select>
        </label>
        <label>Вывод GPIO
          <input type="number" min="0" max="29" bind:value={phy.led_gpio} />
        </label>
        {#if phy.led_driver === 3}
          <label>Порядок цветов
            <select bind:value={phy.led_order}>
              <option value={null}>По умолчанию (GRB)</option>
              {#each LED_ORDERS as o, i}<option value={i}>{o}</option>{/each}
            </select>
          </label>
        {/if}
        <label>Яркость: {phy.led_brightness ?? 15} из 15
          <input type="range" min="1" max="15" value={phy.led_brightness ?? 15}
            oninput={(e) => phy && (phy.led_brightness = +(e.target as HTMLInputElement).value)} />
        </label>
        <label class="check">
          <input type="checkbox" checked={hasOpt(OPT.DIMM)} onchange={(e) => setOpt(OPT.DIMM, (e.target as HTMLInputElement).checked)} />
          Плавное изменение яркости
        </label>
        <label class="check">
          <input type="checkbox" checked={hasOpt(OPT.LED_STEADY)} onchange={(e) => setOpt(OPT.LED_STEADY, (e.target as HTMLInputElement).checked)} />
          Постоянный свет без мигания
        </label>
        <p class="muted">Для платы YD-RP2040: RGB WS2812, GPIO 23, порядок GRB.</p>
        <div class="row">
          <button class="primary" onclick={() => phy && savePhy(phy)} disabled={busy}>Записать</button>
          <button class="ghost" onclick={refresh} disabled={busy}>Отменить изменения</button>
        </div>
      </div>
      <div class="card">
        <h3>Цвета индикатора</h3>
        <ul class="legend">
          <li><i style="background:#22c55e"></i>Зелёный, мигание 0,5 с — ключ подключён и готов.</li>
          <li><i style="background:#22c55e"></i>Зелёный, частое мигание — ключ выполняет операцию.</li>
          <li><i style="background:#eab308"></i>Жёлтый, длинные вспышки — нажмите кнопку BOOT.</li>
          <li><i style="background:#3b82f6"></i>Синий, редкое мигание — компьютер в режиме сна.</li>
          <li><i style="background:#ef4444"></i>Красный, мигание — ключ не подключён к компьютеру (только питание).</li>
        </ul>
      </div>
    {/if}

    {#if tab === "button" && phy}
      <h1>Кнопка присутствия</h1>
      <div class="card form">
        <label class="check big">
          <input type="checkbox" bind:checked={btnEnabled} />
          Требовать нажатие кнопки BOOT
        </label>
        {#if btnEnabled}
          <label>Время ожидания нажатия, секунд
            <input type="number" min="1" max="255" bind:value={btnTimeout} />
          </label>
        {/if}
        <div class="banner {btnEnabled ? 'info' : 'warn'}">
          {#if btnEnabled}
            Ключ ждёт нажатия кнопки BOOT для каждой операции: вход, регистрация, подпись SSH.
            Индикатор мигает, когда ключ ждёт нажатия.
          {:else}
            ⚠️ Кнопка выключена. Ключ подтверждает присутствие автоматически.
            Любая программа на компьютере может использовать ключ без вашего участия. Защиту обеспечивает только PIN.
          {/if}
        </div>
        <p class="muted">
          После включения кнопки запись настроек тоже требует нажатия BOOT.
          Чтобы применить изменение, отключите и подключите ключ.
        </p>
        <button class="primary" onclick={saveButton} disabled={busy}>Записать</button>
      </div>
    {/if}

    {#if tab === "fido"}
      <h1>FIDO и PIN</h1>
      {#if !admin}
        <div class="card">
          <div class="banner warn">
            Windows разрешает операции FIDO (PIN, список passkeys) только программам с правами администратора.
          </div>
          <button class="primary" onclick={() => run(() => api.relaunchAsAdmin())}>Перезапустить от администратора</button>
        </div>
      {:else}
        {#if fido}
          <div class="grid">
            <div class="card stat"><span>PIN</span><b>{fido.pin_set ? "установлен" : "не установлен"}</b></div>
            <div class="card stat"><span>Осталось попыток</span>
              <b class:danger={fido.pin_retries !== null && fido.pin_retries <= 3}>{fido.pin_retries ?? "—"}</b></div>
            <div class="card stat"><span>Свободно мест для passkeys</span><b>{fido.remaining_discoverable || "—"}</b></div>
            <div class="card stat"><span>Версии</span><b class="small-text">{fido.versions.join(", ")}</b></div>
          </div>
        {/if}

        <div class="card form">
          <h3>{fido?.pin_set ? "Изменение PIN" : "Установка PIN"}</h3>
          {#if fido?.pin_set}
            <label>Текущий PIN <input type="password" bind:value={pinCur} autocomplete="off" /></label>
          {/if}
          <label>Новый PIN <input type="password" bind:value={pinNew} autocomplete="off" /></label>
          <label>Подтверждение <input type="password" bind:value={pinNew2} autocomplete="off" /></label>
          <div class="banner warn">
            После 8 неправильных попыток ключ блокируется. Разблокировка — только сброс. Сброс удаляет все passkeys.
          </div>
          <button class="primary" onclick={savePin} disabled={busy}>{fido?.pin_set ? "Изменить PIN" : "Установить PIN"}</button>
        </div>

        <div class="card form">
          <h3>Passkeys на ключе</h3>
          <div class="row">
            <input type="password" placeholder="PIN" bind:value={pinList} autocomplete="off"
              onkeydown={(e) => e.key === "Enter" && loadPasskeys()} />
            <button onclick={loadPasskeys} disabled={busy || !pinList}>Показать</button>
          </div>
          {#if passkeys}
            {#if passkeys.length === 0}
              <p class="muted">На ключе нет passkeys.</p>
            {:else}
              <table>
                <thead><tr><th>Сайт</th><th>Пользователь</th><th></th></tr></thead>
                <tbody>
                  {#each passkeys as p}
                    <tr>
                      <td><b>{p.rp_id}</b>{#if p.rp_name && p.rp_name !== p.rp_id}<div class="muted">{p.rp_name}</div>{/if}</td>
                      <td>{p.user_display_name || p.user_name}{#if p.user_display_name && p.user_name}<div class="muted">{p.user_name}</div>{/if}</td>
                      <td><button class="danger small" onclick={() => removePasskey(p)} disabled={busy}>Удалить</button></td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {/if}
        </div>
      {/if}
    {/if}

    {#if tab === "advanced" && phy}
      <h1>Дополнительно</h1>
      <div class="card form">
        <h3>Интерфейсы USB</h3>
        <label class="check">
          <input type="checkbox" checked={(itf & ITF.KB) !== 0} onchange={(e) => setKb((e.target as HTMLInputElement).checked)} />
          OTP-клавиатура (ключ вводит одноразовый пароль)
        </label>
        <label class="check">
          <input type="checkbox" checked={hasOpt(OPT.DISABLE_POWER_RESET)} onchange={(e) => setOpt(OPT.DISABLE_POWER_RESET, (e.target as HTMLInputElement).checked)} />
          Запретить сброс FIDO после переподключения питания
        </label>
        <p class="muted">Интерфейсы FIDO и смарт-карты всегда включены. Без них утилита не работает с ключом.</p>
        <button class="primary" onclick={() => phy && savePhy(phy)} disabled={busy}>Записать</button>
      </div>
      <div class="card">
        <h3>Конфигурация PHY</h3>
        <pre>{JSON.stringify(phy, null, 2)}</pre>
      </div>
    {/if}

    {#if toast}
      <button class="toast {toast.kind}" onclick={() => (toast = null)}>{toast.text}</button>
    {/if}
    {#if busy}<div class="spinner"></div>{/if}
  </main>
</div>

<style>
  :global(:root) {
    --bg: #0b0d12; --panel: #12151d; --card: #171b25; --line: #252b3a;
    --text: #e6e9ef; --muted: #8a93a6; --accent: #7c5cff; --accent2: #22d3ee;
    --ok: #22c55e; --warn: #eab308; --err: #ef4444;
    font-family: "Segoe UI", Inter, system-ui, sans-serif; font-size: 14px;
    color: var(--text); background: var(--bg);
  }
  :global(body) { margin: 0; }
  .app { display: grid; grid-template-columns: 230px 1fr; height: 100vh; }
  aside { background: var(--panel); border-right: 1px solid var(--line); padding: 18px 12px; display: flex; flex-direction: column; gap: 14px; }
  .brand { display: flex; gap: 10px; align-items: center; padding: 0 6px 10px; }
  .logo { width: 38px; height: 38px; border-radius: 10px; display: grid; place-items: center; font-size: 20px;
    background: linear-gradient(135deg, var(--accent), var(--accent2)); }
  .name { font-weight: 700; font-size: 16px; }
  .sub { color: var(--muted); font-size: 12px; }
  nav { display: flex; flex-direction: column; gap: 4px; flex: 1; }
  nav button { display: flex; gap: 10px; align-items: center; text-align: left; background: transparent; border: 0;
    color: var(--muted); padding: 9px 10px; border-radius: 8px; }
  nav button:hover { background: var(--card); color: var(--text); }
  nav button.active { background: var(--card); color: var(--text); box-shadow: inset 3px 0 0 var(--accent); }
  .status { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--muted); flex-wrap: wrap; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--err); }
  .dot.on { background: var(--ok); box-shadow: 0 0 8px var(--ok); }
  .admin { font-size: 11px; color: var(--muted); }
  .admin.on { color: var(--ok); }
  main { padding: 26px 32px; overflow: auto; position: relative; }
  h1 { margin: 0 0 18px; font-size: 22px; }
  h3 { margin: 0 0 12px; font-size: 15px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 12px; margin-bottom: 12px; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: 12px; padding: 16px 18px; margin-bottom: 12px; }
  .stat { display: flex; flex-direction: column; gap: 6px; margin: 0; }
  .stat span { color: var(--muted); font-size: 12px; }
  .stat b { font-size: 16px; }
  .small-text { font-size: 12px !important; font-weight: 500; }
  .mono { font-family: Consolas, monospace; font-size: 13px !important; }
  .muted { color: var(--muted); font-size: 12px; margin: 8px 0 0; }
  .danger { color: var(--err); }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .form { display: flex; flex-direction: column; gap: 12px; max-width: 560px; }
  label { display: flex; flex-direction: column; gap: 6px; color: var(--muted); font-size: 12px; }
  label.check { flex-direction: row; align-items: center; gap: 8px; color: var(--text); font-size: 13px; }
  label.big { font-size: 15px; }
  input, select { background: #0f1219; border: 1px solid var(--line); color: var(--text); border-radius: 8px; padding: 8px 10px; font: inherit; }
  input[type="checkbox"] { width: 16px; height: 16px; accent-color: var(--accent); }
  input[type="range"] { accent-color: var(--accent); padding: 0; }
  button { background: #222838; color: var(--text); border: 1px solid var(--line); border-radius: 8px; padding: 8px 14px; font: inherit; cursor: pointer; }
  button:hover:not(:disabled) { border-color: var(--accent); }
  button:disabled { opacity: 0.45; cursor: default; }
  button.primary { background: var(--accent); border-color: var(--accent); color: white; align-self: flex-start; }
  button.ghost { background: transparent; }
  button.small { padding: 4px 9px; font-size: 12px; }
  button.danger { background: transparent; border-color: #5b2330; color: var(--err); }
  .bar { height: 8px; background: #0f1219; border-radius: 4px; overflow: hidden; }
  .bar div { height: 100%; background: linear-gradient(90deg, var(--accent), var(--accent2)); }
  .banner { border-radius: 8px; padding: 10px 12px; font-size: 13px; line-height: 1.45; }
  .banner.warn { background: #2a2310; border: 1px solid #5c4a12; color: #f5d77a; }
  .banner.info { background: #10202a; border: 1px solid #164257; color: #9ad9f2; }
  main > .banner { margin-bottom: 14px; }
  .pulse { animation: pulse 1s infinite alternate; }
  @keyframes pulse { to { border-color: var(--warn); box-shadow: 0 0 14px #eab30855; } }
  .legend { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .legend i { display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 10px; }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; color: var(--muted); font-weight: 500; font-size: 12px; padding: 6px 4px; border-bottom: 1px solid var(--line); }
  td { padding: 8px 4px; border-bottom: 1px solid var(--line); vertical-align: top; }
  td:last-child { text-align: right; }
  pre { background: #0f1219; padding: 12px; border-radius: 8px; font-size: 12px; overflow: auto; margin: 0; }
  .toast { position: fixed; right: 20px; bottom: 20px; max-width: 460px; text-align: left; border-radius: 10px; padding: 12px 14px; line-height: 1.4; }
  .toast.ok { background: #0f2a1a; border-color: #1f6b3a; }
  .toast.err { background: #2a1015; border-color: #6b1f2b; }
  .toast.info { background: #10202a; border-color: #164257; }
  .spinner { position: fixed; top: 16px; right: 20px; width: 18px; height: 18px; border: 2px solid var(--line); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
