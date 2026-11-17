(() => {
  const accountLink = 'Manage your Google Account';
  const interactive = 'a, button, [role="button"]';
  const hiddenMarker = 'data-gmaildock-hidden';
  let hideAccountLinkEnabled = true;

  function hideAccountLink(element) {
    if (!hideAccountLinkEnabled || element.hasAttribute(hiddenMarker)) return;
    if (element.textContent?.trim() !== accountLink) return;
    element.setAttribute(hiddenMarker, '');
    element.setAttribute('data-gmaildock-display', element.style.getPropertyValue('display'));
    element.setAttribute('data-gmaildock-priority', element.style.getPropertyPriority('display'));
    element.style.setProperty('display', 'none', 'important');
  }

  function scan(root) {
    if (!hideAccountLinkEnabled) return;
    if (root.nodeType === Node.TEXT_NODE) {
      const parent = root.parentElement?.closest(interactive);
      if (parent) hideAccountLink(parent);
      return;
    }
    if (root.nodeType !== Node.ELEMENT_NODE) return;
    if (root.matches(interactive)) hideAccountLink(root);
    for (const element of root.querySelectorAll(interactive)) hideAccountLink(element);
  }

  function notifyChildren() {
    for (const frame of document.querySelectorAll('iframe')) {
      frame.contentWindow?.postMessage({ type: 'gmaildock:hide-link', value: hideAccountLinkEnabled }, '*');
    }
  }

  function setHideAccountLink(value) {
    hideAccountLinkEnabled = value;
    if (value) scan(document.documentElement);
    else {
      for (const element of document.querySelectorAll(`[${hiddenMarker}]`)) {
        const display = element.getAttribute('data-gmaildock-display');
        const priority = element.getAttribute('data-gmaildock-priority');
        if (display) element.style.setProperty('display', display, priority || '');
        else element.style.removeProperty('display');
        element.removeAttribute(hiddenMarker);
        element.removeAttribute('data-gmaildock-display');
        element.removeAttribute('data-gmaildock-priority');
      }
    }
    notifyChildren();
  }

  window.addEventListener('message', (event) => {
    if (event.data?.type === 'gmaildock:request-hide-link' && event.source) {
      event.source.postMessage({ type: 'gmaildock:hide-link', value: hideAccountLinkEnabled }, event.origin || '*');
    } else if (event.data?.type === 'gmaildock:hide-link' && event.source === window.parent) {
      setHideAccountLink(Boolean(event.data.value));
    }
  });

  function mount() {
    scan(document.documentElement);
    new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) scan(node);
      }
    }).observe(document.documentElement, { childList: true, subtree: true });

    if (window !== window.top) {
      window.parent.postMessage({ type: 'gmaildock:request-hide-link' }, '*');
      return;
    }
    if (location.hostname !== 'mail.google.com') return;

    const host = document.createElement('div');
    host.id = 'gmaildock-controls';
    Object.assign(host.style, { position: 'fixed', right: '18px', bottom: '18px', zIndex: '2147483647' });
    const ui = host.attachShadow({ mode: 'closed' });
    ui.innerHTML = `
      <style>
        :host { font: 13px/1.4 system-ui, sans-serif; color: #27343b; }
        * { box-sizing: border-box; }
        [hidden] { display: none !important; }
        button { font: inherit; cursor: pointer; }
        button:focus-visible, input:focus-visible { outline: 2px solid #087e78; outline-offset: 3px; }
        .bar { display: flex; justify-content: flex-end; gap: 8px; }
        .pill { min-height: 40px; padding: 0 16px; border: 1px solid #e7e9eb; border-radius: 22px;
          background: #fff; color: #27343b; box-shadow: 0 4px 18px #0002; font-weight: 600; }
        .pill:hover { background: #f6f9f8; }
        .update { background: #d6f4e9; border-color: #b6e6d3; }
        .settings { display: flex; align-items: center; gap: 7px; }
        .settings svg { width: 16px; height: 16px; }
        .panel { position: absolute; right: 0; bottom: calc(100% + 10px);
          width: min(304px, calc(100vw - 36px)); padding: 18px; border: 1px solid #e4e9e8;
          border-radius: 18px; background: #fff; box-shadow: 0 12px 38px #182a3440; }
        h2 { margin: 0 0 15px; font-size: 17px; line-height: 1.2; }
        .option { display: flex; align-items: center; justify-content: space-between; gap: 15px;
          min-height: 48px; border-top: 1px solid #eef1f1; }
        .option span { max-width: 220px; }
        input { width: 17px; height: 17px; accent-color: #087e78; cursor: pointer; }
        .check { width: 100%; margin-top: 13px; padding: 10px; border: 1px solid #d5e4e1;
          border-radius: 10px; background: #f1f8f6; color: #086f69; font-weight: 600; }
        .check:hover { background: #e4f2ee; }
        .status { min-height: 18px; margin: 9px 0 0; color: #64737a; font-size: 12px; }
      </style>
      <div class="bar">
        <button class="pill update" id="update" type="button" hidden></button>
        <button class="pill settings" id="settings" type="button" aria-label="GmailDock settings" aria-expanded="false">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M10 2h4l.6 2.2 1.8.8 2-.9 2.8 2.8-.9 2 0 0 .8 1.8L23 11v4l-2.2.6-.8 1.8.9 2-2.8 2.8-2-.9-1.8.8L14 24h-4l-.6-2.2-1.8-.8-2 .9-2.8-2.8.9-2-.8-1.8L1 15v-4l2.2-.6.8-1.8-.9-2L5.9 3.8l2 .9 1.8-.8L10 2Z" transform="translate(0 -1) scale(.96)"/><circle cx="12" cy="12" r="3"/></svg>
          Settings
        </button>
      </div>
      <section class="panel" id="panel" aria-label="GmailDock settings" hidden>
        <h2>GmailDock settings</h2>
        <label class="option"><span>Check for updates automatically</span><input id="auto-updates" type="checkbox" checked></label>
        <label class="option"><span>Hide Google Account link</span><input id="hide-link" type="checkbox" checked></label>
        <button class="check" id="check-now" type="button">Check for updates now</button>
        <p class="status" id="status" role="status" aria-live="polite"></p>
      </section>`;
    document.body.append(host);

    const settingsButton = ui.getElementById('settings');
    const panel = ui.getElementById('panel');
    const updateButton = ui.getElementById('update');
    const autoUpdates = ui.getElementById('auto-updates');
    const hideLink = ui.getElementById('hide-link');
    const status = ui.getElementById('status');
    let settings = { autoCheckUpdates: true, hideAccountLink: true };
    let availableVersion;
    let checking = false;
    let startupCheck;
    let periodicCheck;

    function scheduleChecks() {
      clearTimeout(startupCheck);
      clearInterval(periodicCheck);
      if (!settings.autoCheckUpdates) return;
      startupCheck = setTimeout(() => checkUpdates(false), 8000);
      periodicCheck = setInterval(() => checkUpdates(false), 30 * 60 * 1000);
    }

    async function checkUpdates(manual) {
      if (checking || updateButton.disabled) return;
      checking = true;
      if (manual) status.textContent = 'Checking for updates…';
      try {
        const version = await window.__TAURI__.core.invoke('check_for_updates');
        availableVersion = version;
        updateButton.hidden = !version;
        if (version) {
          updateButton.textContent = `Update to v${version}`;
          updateButton.title = `Install GmailDock v${version}`;
        }
        if (manual) status.textContent = version ? `Version ${version} is available.` : 'You are up to date.';
      } catch (error) {
        if (manual) status.textContent = `Update check failed: ${error}`;
      } finally {
        checking = false;
      }
    }

    async function changeSetting(key, value) {
      const previous = settings;
      settings = { ...settings, [key]: value };
      if (key === 'hideAccountLink') setHideAccountLink(value);
      else scheduleChecks();
      autoUpdates.disabled = true;
      hideLink.disabled = true;
      try {
        await window.__TAURI__.core.invoke('save_settings', { settings });
        status.textContent = 'Settings saved.';
      } catch (error) {
        settings = previous;
        autoUpdates.checked = settings.autoCheckUpdates;
        hideLink.checked = settings.hideAccountLink;
        setHideAccountLink(settings.hideAccountLink);
        scheduleChecks();
        status.textContent = `Could not save settings: ${error}`;
      } finally {
        autoUpdates.disabled = false;
        hideLink.disabled = false;
      }
    }

    settingsButton.addEventListener('click', () => {
      panel.hidden = !panel.hidden;
      settingsButton.setAttribute('aria-expanded', String(!panel.hidden));
    });
    document.addEventListener('pointerdown', (event) => {
      if (!host.contains(event.target)) {
        panel.hidden = true;
        settingsButton.setAttribute('aria-expanded', 'false');
      }
    });
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && !panel.hidden) {
        panel.hidden = true;
        settingsButton.setAttribute('aria-expanded', 'false');
        settingsButton.focus();
      }
    });
    autoUpdates.addEventListener('change', () => changeSetting('autoCheckUpdates', autoUpdates.checked));
    hideLink.addEventListener('change', () => changeSetting('hideAccountLink', hideLink.checked));
    ui.getElementById('check-now').addEventListener('click', () => checkUpdates(true));
    updateButton.addEventListener('click', async () => {
      if (!availableVersion || !confirm(`Install GmailDock v${availableVersion} now?`)) return;
      updateButton.disabled = true;
      updateButton.textContent = 'Installing update…';
      try {
        await window.__TAURI__.core.invoke('install_update');
      } catch (error) {
        updateButton.disabled = false;
        updateButton.textContent = 'Update failed — retry';
        status.textContent = `Could not install update: ${error}`;
      }
    });

    window.__TAURI__.core.invoke('get_settings').then((saved) => {
      settings = saved;
      autoUpdates.checked = settings.autoCheckUpdates;
      hideLink.checked = settings.hideAccountLink;
      setHideAccountLink(settings.hideAccountLink);
      scheduleChecks();
    }).catch((error) => {
      status.textContent = `Could not load settings: ${error}`;
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount, { once: true });
  } else {
    mount();
  }
})();
