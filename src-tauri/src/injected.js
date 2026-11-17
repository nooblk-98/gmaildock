(() => {
  const accountLink = 'Manage your Google Account';
  const interactive = 'a, button, [role="button"]';

  function hideAccountLink(element) {
    if (element.textContent?.trim() === accountLink) {
      element.style.setProperty('display', 'none', 'important');
    }
  }

  function hideGoogleApps(element) {
    if (element.getAttribute('aria-label') !== 'Google apps' && element.getAttribute('title') !== 'Google apps') return;
    const button = element.closest(interactive) || element;
    button.style.setProperty('display', 'none', 'important');
  }

  function scan(root) {
    if (root.nodeType === Node.TEXT_NODE) {
      const parent = root.parentElement?.closest(interactive);
      if (parent) hideAccountLink(parent);
      return;
    }
    if (root.nodeType !== Node.ELEMENT_NODE) return;
    if (root.matches(interactive)) hideAccountLink(root);
    hideGoogleApps(root);
    for (const element of root.querySelectorAll(`${interactive}, [aria-label="Google apps"], [title="Google apps"]`)) {
      hideAccountLink(element);
      hideGoogleApps(element);
    }
  }

  function mount() {
    scan(document.documentElement);
    new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) scan(node);
      }
    }).observe(document.documentElement, { childList: true, subtree: true });

    if (window !== window.top) return;
    if (location.hostname !== 'mail.google.com') return;

    const make = (tag, text) => {
      const element = document.createElement(tag);
      if (text) element.textContent = text;
      return element;
    };
    const host = make('div');
    host.id = 'gmaildock-controls';
    Object.assign(host.style, { position: 'fixed', right: '18px', bottom: '18px', zIndex: '2147483647', font: '13px system-ui, sans-serif', color: '#27343b' });
    const ui = host.attachShadow({ mode: 'open' });
    const bar = make('div');
    Object.assign(bar.style, { display: 'flex', justifyContent: 'flex-end', gap: '8px' });

    function stylePill(button) {
      button.type = 'button';
      Object.assign(button.style, { minHeight: '40px', padding: '0 16px', border: '1px solid #e7e9eb', borderRadius: '22px', background: '#fff', color: '#27343b', boxShadow: '0 4px 18px #0002', font: '600 13px system-ui, sans-serif', cursor: 'pointer' });
    }

    const updateButton = make('button');
    stylePill(updateButton);
    Object.assign(updateButton.style, { display: 'none', background: '#d6f4e9', borderColor: '#b6e6d3' });
    const settingsButton = make('button', '⚙ Settings');
    stylePill(settingsButton);
    settingsButton.setAttribute('aria-label', 'GmailDock settings');
    settingsButton.setAttribute('aria-expanded', 'false');
    bar.append(updateButton, settingsButton);

    const panel = make('section');
    panel.setAttribute('aria-label', 'GmailDock settings');
    Object.assign(panel.style, { display: 'none', position: 'absolute', right: '0', bottom: '50px', width: 'min(304px, calc(100vw - 36px))', padding: '18px', border: '1px solid #e4e9e8', borderRadius: '18px', background: '#fff', boxShadow: '0 12px 38px #182a3440' });
    const heading = make('h2', 'GmailDock settings');
    Object.assign(heading.style, { margin: '0 0 15px', font: '600 17px system-ui, sans-serif' });
    panel.append(heading);

    function addOption(labelText) {
      const label = make('label');
      Object.assign(label.style, { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '15px', minHeight: '48px', borderTop: '1px solid #eef1f1', cursor: 'pointer' });
      const caption = make('span', labelText);
      const checkbox = make('input');
      checkbox.type = 'checkbox';
      checkbox.checked = true;
      Object.assign(checkbox.style, { width: '17px', height: '17px', accentColor: '#087e78', cursor: 'pointer' });
      label.append(caption, checkbox);
      panel.append(label);
      return checkbox;
    }

    const autoUpdates = addOption('Check for updates automatically');
    const closeToTray = addOption('Keep running in tray when closing');
    const checkNow = make('button', 'Check for updates now');
    checkNow.type = 'button';
    Object.assign(checkNow.style, { width: '100%', marginTop: '13px', padding: '10px', border: '1px solid #d5e4e1', borderRadius: '10px', background: '#f1f8f6', color: '#086f69', font: '600 13px system-ui, sans-serif', cursor: 'pointer' });
    const status = make('p');
    status.setAttribute('role', 'status');
    status.setAttribute('aria-live', 'polite');
    Object.assign(status.style, { minHeight: '18px', margin: '9px 0 0', color: '#64737a', font: '12px system-ui, sans-serif' });
    panel.append(checkNow, status);
    ui.append(panel, bar);
    document.body.append(host);
    let settings = { autoCheckUpdates: true, closeToTray: true };
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
        updateButton.style.display = version ? 'inline-flex' : 'none';
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
      if (key === 'autoCheckUpdates') scheduleChecks();
      autoUpdates.disabled = true;
      closeToTray.disabled = true;
      try {
        await window.__TAURI__.core.invoke('save_settings', { settings });
        status.textContent = 'Settings saved.';
      } catch (error) {
        settings = previous;
        autoUpdates.checked = settings.autoCheckUpdates;
        closeToTray.checked = settings.closeToTray;
        scheduleChecks();
        status.textContent = `Could not save settings: ${error}`;
      } finally {
        autoUpdates.disabled = false;
        closeToTray.disabled = false;
      }
    }

    function setPanelOpen(open) {
      panel.style.display = open ? 'block' : 'none';
      settingsButton.setAttribute('aria-expanded', String(open));
    }
    settingsButton.addEventListener('click', () => setPanelOpen(panel.style.display === 'none'));
    document.addEventListener('pointerdown', (event) => {
      if (!host.contains(event.target)) setPanelOpen(false);
    });
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && panel.style.display !== 'none') {
        setPanelOpen(false);
        settingsButton.focus();
      }
    });
    autoUpdates.addEventListener('change', () => changeSetting('autoCheckUpdates', autoUpdates.checked));
    closeToTray.addEventListener('change', () => changeSetting('closeToTray', closeToTray.checked));
    checkNow.addEventListener('click', () => checkUpdates(true));
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
      closeToTray.checked = settings.closeToTray;
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
