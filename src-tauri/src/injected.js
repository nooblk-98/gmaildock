(() => {
  if (window !== window.top) return;
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

    if (location.hostname !== 'mail.google.com') return;
    if (!document.body) return;

    let unreadCount;
    let missingSince;
    // <unread-parse>
    function parseInboxCount(text) {
      const match = /(?:^|\D)(\d[\d,]*)/.exec(text || '');
      return match ? Number(match[1].replaceAll(',', '')) : 0;
    }
    // </unread-parse>
    function findInboxLink() {
      const links = [...document.querySelectorAll('a[href$="#inbox"]')];
      const inNav = links.find((link) => link.closest('[role="navigation"], .aeN, .wT'));
      if (inNav || links.length === 1) return inNav || links[0];
      // Fallback for markup changes: any navigation entry labelled "Inbox".
      return document.querySelector('[role="navigation"] [aria-label^="Inbox"], [role="navigation"] a[title^="Inbox"]');
    }
    function inboxUnread() {
      // Only the Inbox sidebar entry counts; other labels (Spam, Drafts, ...) are ignored.
      const link = findInboxLink();
      if (!link) return undefined;
      return parseInboxCount(`${link.getAttribute('aria-label') || ''} ${link.textContent || ''}`);
    }
    function syncUnreadCount() {
      const found = inboxUnread();
      // Sidebar not rendered yet: keep the last known value instead of falling back to the tab title,
      // because the title shows the count of whichever view is open.
      if (found === undefined) {
        missingSince ??= Date.now();
        if (Date.now() - missingSince > 60000 && missingSince !== -1) {
          missingSince = -1;
          console.warn('GmailDock: could not find the Inbox entry; the unread badge may be out of date.');
        }
        return;
      }
      missingSince = undefined;
      if (!Number.isSafeInteger(found) || found === unreadCount) return;
      unreadCount = found;
      window.__TAURI__.core.invoke('set_unread_count', { count: found }).catch((error) => {
        console.warn('Could not set GmailDock unread badge:', error);
      });
    }
    new MutationObserver(syncUnreadCount).observe(document.body, { childList: true, subtree: true, characterData: true });
    syncUnreadCount();
    setInterval(syncUnreadCount, 5000);

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
    updateButton.style.display = 'none';
    bar.append(updateButton);

    const panel = make('section');
    panel.setAttribute('aria-label', 'GmailDock settings');
    Object.assign(panel.style, { display: 'none', position: 'absolute', right: '0', bottom: '50px', width: 'min(304px, calc(100vw - 36px))', padding: '18px', border: '1px solid #e4e9e8', borderRadius: '18px', background: '#fff', boxShadow: '0 12px 38px #182a3440' });
    const heading = make('h2', 'GmailDock settings');
    Object.assign(heading.style, { margin: '0 0 15px', font: '600 17px system-ui, sans-serif' });
    const closePanel = make('button', 'Close');
    closePanel.type = 'button';
    Object.assign(closePanel.style, { float: 'right', padding: '2px 8px', border: '0', background: 'transparent', color: '#086f69', font: '600 12px system-ui, sans-serif', cursor: 'pointer' });
    closePanel.addEventListener('click', () => setPanelOpen(false));
    panel.append(closePanel, heading);

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
    const startAtLogin = addOption('Start automatically at login');
    const startMinimized = addOption('Start minimized to tray at login');
    const notifications = addOption('Notify me about new mail');
    const externalLinks = addOption('Open non-Google links in my browser');
    const options = [autoUpdates, closeToTray, startAtLogin, startMinimized, notifications, externalLinks];

    const zoomRow = make('div');
    Object.assign(zoomRow.style, { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '10px', minHeight: '48px', borderTop: '1px solid #eef1f1' });
    const zoomLabel = make('span', 'Zoom');
    const zoomControls = make('div');
    Object.assign(zoomControls.style, { display: 'flex', alignItems: 'center', gap: '8px' });
    const zoomOut = make('button', '−');
    const zoomValue = make('span', '100%');
    const zoomIn = make('button', '+');
    for (const button of [zoomOut, zoomIn]) {
      button.type = 'button';
      Object.assign(button.style, { width: '28px', height: '28px', border: '1px solid #d5e4e1', borderRadius: '8px', background: '#f1f8f6', color: '#086f69', font: '600 15px system-ui, sans-serif', cursor: 'pointer' });
    }
    zoomOut.setAttribute('aria-label', 'Zoom out');
    zoomIn.setAttribute('aria-label', 'Zoom in');
    Object.assign(zoomValue.style, { minWidth: '40px', textAlign: 'center' });
    zoomControls.append(zoomOut, zoomValue, zoomIn);
    zoomRow.append(zoomLabel, zoomControls);
    panel.append(zoomRow);

    const shortcutHint = make('p', 'Tip: press Ctrl+Shift+G (⌘⇧G on macOS) to show or hide GmailDock.');
    Object.assign(shortcutHint.style, { margin: '10px 0 0', color: '#64737a', font: '12px system-ui, sans-serif' });
    panel.append(shortcutHint);
    const checkNow = make('button', 'Check for updates now');
    checkNow.type = 'button';
    Object.assign(checkNow.style, { width: '100%', marginTop: '13px', padding: '10px', border: '1px solid #d5e4e1', borderRadius: '10px', background: '#f1f8f6', color: '#086f69', font: '600 13px system-ui, sans-serif', cursor: 'pointer' });
    const status = make('p');
    status.setAttribute('role', 'status');
    status.setAttribute('aria-live', 'polite');
    Object.assign(status.style, { minHeight: '18px', margin: '9px 0 0', color: '#64737a', font: '12px system-ui, sans-serif' });
    panel.append(checkNow, status);

    const aboutHeading = make('h3', 'About');
    Object.assign(aboutHeading.style, { margin: '18px 0 8px', paddingTop: '16px', borderTop: '1px solid #eef1f1', font: '600 15px system-ui, sans-serif' });
    const appVersion = make('p', 'GmailDock');
    Object.assign(appVersion.style, { margin: '0 0 8px', color: '#64737a' });
    const developer = make('p', 'Developer: Lahiru S Liyanage');
    Object.assign(developer.style, { margin: '0 0 8px' });
    const aboutLinks = make('div');
    Object.assign(aboutLinks.style, { display: 'flex', gap: '14px' });
    const developerLink = make('button', 'GitHub profile ↗');
    const repositoryLink = make('button', 'Source repository ↗');
    for (const link of [developerLink, repositoryLink]) {
      link.type = 'button';
      Object.assign(link.style, { padding: '0', border: '0', background: 'transparent', color: '#087e78', font: '600 12px system-ui, sans-serif', cursor: 'pointer', textDecoration: 'underline' });
    }
    aboutLinks.append(developerLink, repositoryLink);
    panel.append(aboutHeading, appVersion, developer, aboutLinks);

    const overlay = make('div');
    Object.assign(overlay.style, { display: 'none', position: 'fixed', inset: '0', alignItems: 'center', justifyContent: 'center', padding: '24px', background: '#172b3699', zIndex: '1' });
    const dialog = make('section');
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    dialog.setAttribute('aria-label', 'GmailDock update available');
    Object.assign(dialog.style, { width: 'min(440px, 100%)', maxHeight: 'min(80vh, 620px)', overflow: 'auto', padding: '26px', borderRadius: '20px', background: '#fff', boxShadow: '0 24px 70px #0005' });
    const updateHeading = make('h2');
    Object.assign(updateHeading.style, { margin: '0 0 8px', font: '700 22px system-ui, sans-serif' });
    const updateIntro = make('p', 'What’s new');
    Object.assign(updateIntro.style, { margin: '0 0 12px', color: '#5b6b72', font: '13px system-ui, sans-serif' });
    const releaseNotes = make('div');
    Object.assign(releaseNotes.style, { maxHeight: '240px', overflow: 'auto', padding: '14px', borderRadius: '12px', background: '#f4f7f6', whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', lineHeight: '1.5', font: '13px system-ui, sans-serif' });
    const updateStatus = make('p');
    updateStatus.setAttribute('role', 'status');
    Object.assign(updateStatus.style, { minHeight: '18px', margin: '12px 0 0', color: '#8b3333', font: '12px system-ui, sans-serif' });
    const updateActions = make('div');
    Object.assign(updateActions.style, { display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '18px' });
    const laterButton = make('button', 'Later');
    stylePill(laterButton);
    const installButton = make('button', 'Update now');
    stylePill(installButton);
    Object.assign(installButton.style, { background: '#087e78', borderColor: '#087e78', color: '#fff' });
    updateActions.append(laterButton, installButton);
    dialog.append(updateHeading, updateIntro, releaseNotes, updateStatus, updateActions);
    overlay.append(dialog);
    ui.append(panel, bar, overlay);
    document.body.append(host);
    let settings = { autoCheckUpdates: true, closeToTray: true, startAtLogin: true, notifications: true, startMinimized: false, openExternalInBrowser: true, zoom: 1 };
    let availableUpdate;
    let announcedVersion;
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

    function showUpdateDialog() {
      if (!availableUpdate) return;
      setPanelOpen(false);
      updateHeading.textContent = `GmailDock v${availableUpdate.version} is available`;
      releaseNotes.textContent = availableUpdate.notes?.trim() || 'Release notes are unavailable.';
      updateStatus.textContent = '';
      installButton.textContent = 'Update now';
      installButton.disabled = false;
      laterButton.disabled = false;
      overlay.style.display = 'flex';
      installButton.focus();
    }

    function hideUpdateDialog() {
      if (installButton.disabled) return;
      overlay.style.display = 'none';
      updateButton.focus();
    }

    async function checkUpdates(manual) {
      if (checking || updateButton.disabled) return;
      checking = true;
      if (manual) status.textContent = 'Checking for updates…';
      try {
        const update = await window.__TAURI__.core.invoke('check_for_updates');
        availableUpdate = update;
        updateButton.style.display = update ? 'inline-flex' : 'none';
        if (update) {
          updateButton.textContent = `Update to v${update.version}`;
          updateButton.title = `See what’s new in GmailDock v${update.version}`;
          if (manual || announcedVersion !== update.version) {
            announcedVersion = update.version;
            showUpdateDialog();
          }
        } else {
          overlay.style.display = 'none';
        }
        if (manual) status.textContent = update ? `Version ${update.version} is available.` : 'You are up to date.';
      } catch (error) {
        if (manual) status.textContent = `Update check failed: ${error}`;
      } finally {
        checking = false;
      }
    }

    function syncControls() {
      autoUpdates.checked = settings.autoCheckUpdates;
      closeToTray.checked = settings.closeToTray;
      startAtLogin.checked = settings.startAtLogin;
      startMinimized.checked = settings.startMinimized;
      notifications.checked = settings.notifications;
      externalLinks.checked = settings.openExternalInBrowser;
      zoomValue.textContent = `${Math.round(settings.zoom * 100)}%`;
    }

    function stepZoom(delta) {
      const next = Math.min(2, Math.max(0.5, Math.round((settings.zoom + delta) * 100) / 100));
      if (next !== settings.zoom) changeSetting('zoom', next);
    }

    async function changeSetting(key, value) {
      const previous = settings;
      settings = { ...settings, [key]: value };
      syncControls();
      if (key === 'autoCheckUpdates') scheduleChecks();
      options.forEach((option) => { option.disabled = true; });
      try {
        await window.__TAURI__.core.invoke('save_settings', { settings });
        status.textContent = 'Settings saved.';
      } catch (error) {
        settings = previous;
        syncControls();
        scheduleChecks();
        status.textContent = `Could not save settings: ${error}`;
      } finally {
        options.forEach((option) => { option.disabled = false; });
      }
    }

    function setPanelOpen(open) {
      panel.style.display = open ? 'block' : 'none';
    }
    // Opened from the tray menu (Settings).
    window.__gmaildockOpenSettings = () => setPanelOpen(true);
    document.addEventListener('pointerdown', (event) => {
      if (!host.contains(event.target)) setPanelOpen(false);
    });
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && overlay.style.display !== 'none') {
        hideUpdateDialog();
        return;
      }
      if (event.key === 'Escape' && panel.style.display !== 'none') {
        setPanelOpen(false);
      }
    });
    autoUpdates.addEventListener('change', () => changeSetting('autoCheckUpdates', autoUpdates.checked));
    closeToTray.addEventListener('change', () => changeSetting('closeToTray', closeToTray.checked));
    startAtLogin.addEventListener('change', () => changeSetting('startAtLogin', startAtLogin.checked));
    startMinimized.addEventListener('change', () => changeSetting('startMinimized', startMinimized.checked));
    notifications.addEventListener('change', () => changeSetting('notifications', notifications.checked));
    externalLinks.addEventListener('change', () => changeSetting('openExternalInBrowser', externalLinks.checked));
    zoomOut.addEventListener('click', () => stepZoom(-0.1));
    zoomIn.addEventListener('click', () => stepZoom(0.1));
    checkNow.addEventListener('click', () => checkUpdates(true));
    developerLink.addEventListener('click', () => window.__TAURI__.core.invoke('open_about_link', { link: 'developer' }).catch((error) => { status.textContent = `Could not open link: ${error}`; }));
    repositoryLink.addEventListener('click', () => window.__TAURI__.core.invoke('open_about_link', { link: 'repository' }).catch((error) => { status.textContent = `Could not open link: ${error}`; }));
    updateButton.addEventListener('click', showUpdateDialog);
    laterButton.addEventListener('click', hideUpdateDialog);
    installButton.addEventListener('click', async () => {
      if (!availableUpdate) return;
      updateButton.disabled = true;
      installButton.disabled = true;
      laterButton.disabled = true;
      updateButton.textContent = 'Installing update…';
      installButton.textContent = 'Installing…';
      updateStatus.textContent = 'Downloading and installing the update…';
      try {
        await window.__TAURI__.core.invoke('install_update');
      } catch (error) {
        updateButton.disabled = false;
        installButton.disabled = false;
        laterButton.disabled = false;
        updateButton.textContent = `Update to v${availableUpdate.version}`;
        installButton.textContent = 'Retry update';
        updateStatus.textContent = `Could not install update: ${error}`;
      }
    });

    window.__TAURI__.core.invoke('get_settings').then((saved) => {
      settings = { ...settings, ...saved };
      syncControls();
      scheduleChecks();
    }).catch((error) => {
      status.textContent = `Could not load settings: ${error}`;
    });
    window.__TAURI__.core.invoke('get_app_version').then((version) => {
      appVersion.textContent = `GmailDock v${version}`;
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount, { once: true });
  } else {
    mount();
  }
})();
