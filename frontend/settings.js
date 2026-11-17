(() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  const el = (id) => document.getElementById(id);
  const checkboxSettings = {
    autoCheckUpdates: 'autoCheckUpdates',
    closeToTray: 'closeToTray',
    notifications: 'notifications',
    openExternalInBrowser: 'openExternalInBrowser',
    startAtLogin: 'startAtLogin',
    startMinimized: 'startMinimized',
  };

  let settings;
  let availableUpdate = null;
  let installing = false;
  let downloaded = 0;
  let saveQueue = Promise.resolve();
  let statusTimer;

  function setStatus(message, isError = false) {
    const status = el('status');
    status.textContent = message;
    status.classList.toggle('error', isError);
    clearTimeout(statusTimer);
    if (message && !isError) {
      statusTimer = setTimeout(() => {
        status.textContent = '';
      }, 2500);
    }
  }

  function setUpdateStatus(message, isError = false) {
    const status = el('updateStatus');
    status.textContent = message;
    status.classList.toggle('error', isError);
  }

  function setZoomValue(zoom) {
    const select = el('zoom');
    const known = [...select.options].some((option) => Number(option.value) === zoom);
    if (!known) select.add(new Option(`${Math.round(zoom * 100)}%`, String(zoom)));
    select.value = String(zoom);
  }

  function render() {
    for (const [key, id] of Object.entries(checkboxSettings)) el(id).checked = Boolean(settings[key]);
    setZoomValue(settings.zoom);
    el('shortcut').value = settings.shortcut ?? '';
    renderUpdate();
  }

  function renderUpdate() {
    const version = availableUpdate?.version;
    el('updateBox').hidden = !version || installing || version === settings.skippedVersion;
    el('skippedNote').hidden = !settings.skippedVersion;
    el('skippedVersion').textContent = settings.skippedVersion ?? '';
    if (version) {
      el('updateVersion').textContent = `GmailDock ${version}`;
      el('updateNotes').textContent =
        availableUpdate.notes?.trim() || 'Release notes are unavailable.';
    }
  }

  async function load() {
    settings = await invoke('get_settings');
    render();
  }

  function save(patch) {
    settings = { ...settings, ...patch };
    render();

    const payload = settings;
    saveQueue = saveQueue.then(async () => {
      try {
        settings = await invoke('save_settings', { settings: payload });
        render();
        setStatus('Settings saved.');
      } catch (error) {
        setStatus(String(error), true);
        await load();
      }
    });
  }

  async function checkNow() {
    el('checkNow').disabled = true;
    setUpdateStatus('Checking for updates…');
    try {
      availableUpdate = await invoke('check_for_updates');
      renderUpdate();
      setUpdateStatus(
        availableUpdate ? `Version ${availableUpdate.version} is available.` : 'You are up to date.',
      );
    } catch (error) {
      setUpdateStatus(`Update check failed: ${error}`, true);
    } finally {
      el('checkNow').disabled = false;
    }
  }

  async function install() {
    if (!availableUpdate) return;

    installing = true;
    downloaded = 0;
    el('updateProgress').hidden = false;
    el('updateProgress').value = 0;
    el('installButton').disabled = true;
    el('skipButton').disabled = true;
    setUpdateStatus('Downloading and installing…');

    try {
      await invoke('install_update');
    } catch (error) {
      installing = false;
      el('installButton').disabled = false;
      el('skipButton').disabled = false;
      setUpdateStatus(`Could not install the update: ${error}`, true);
      renderUpdate();
    }
  }

  function openAbout(link) {
    invoke('open_about_link', { link }).catch((error) => {
      setStatus(`Could not open the link: ${error}`, true);
    });
  }

  function bindEvents() {
    for (const [key, id] of Object.entries(checkboxSettings)) {
      el(id).addEventListener('change', () => save({ [key]: el(id).checked }));
    }
    el('zoom').addEventListener('change', () => save({ zoom: Number(el('zoom').value) }));
    el('shortcut').addEventListener('change', () =>
      save({ shortcut: el('shortcut').value.trim() || null }),
    );

    el('checkNow').addEventListener('click', checkNow);
    el('installButton').addEventListener('click', install);
    el('skipButton').addEventListener('click', () =>
      save({ skippedVersion: availableUpdate?.version ?? null }),
    );
    el('unskip').addEventListener('click', () => save({ skippedVersion: null }));
    el('developerLink').addEventListener('click', () => openAbout('developer'));
    el('repositoryLink').addEventListener('click', () => openAbout('repository'));
  }

  async function bindUpdates() {
    availableUpdate = await invoke('get_update_state').catch(() => null);
    renderUpdate();

    await listen('update-available', (event) => {
      availableUpdate = event.payload;
      renderUpdate();
      setUpdateStatus(`Version ${availableUpdate.version} is available.`);
    }).catch(() => {});

    await listen('update-progress', (event) => {
      const { chunk, total } = event.payload;
      downloaded += chunk;

      const progress = el('updateProgress');
      progress.hidden = false;
      if (total) {
        progress.value = Math.min(100, Math.round((downloaded / total) * 100));
        setUpdateStatus(`Downloading… ${progress.value}%`);
      } else {
        progress.removeAttribute('value');
        setUpdateStatus('Downloading…');
      }
    }).catch(() => {});

    if (!availableUpdate && settings.autoCheckUpdates) checkNow();
  }

  async function init() {
    try {
      el('version').textContent = `Version ${await invoke('get_app_version')}`;
      await load();
    } catch (error) {
      setStatus(`Could not load settings: ${error}`, true);
      return;
    }

    bindEvents();
    await bindUpdates();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init, { once: true });
  } else {
    init();
  }
})();
