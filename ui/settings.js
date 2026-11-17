const invoke = window.__TAURI__.core.invoke;
const fields = {
  startAtLogin: 'Start at login', startMinimized: 'Start minimized',
  closeToTray: 'Keep running in the tray when closed', notifications: 'New mail notifications',
  openExternalInBrowser: 'Open non-Google links in my browser', autoCheckUpdates: 'Check for updates automatically',
};
let current;
for (const [key, name] of Object.entries(fields)) {
  const label = document.createElement('label');
  const input = document.createElement('input'); input.type = 'checkbox'; input.id = key;
  label.append(input, ` ${name}`); document.querySelector('#toggles').append(label);
}
const status = document.querySelector('#status');
async function load() {
  const data = await invoke('get_preferences'); current = data.settings;
  for (const key of Object.keys(fields)) document.getElementById(key).checked = current[key];
  document.querySelector('#shortcut').value = current.shortcut || '';
  document.querySelector('#zoom').value = current.zoom;
  document.querySelector('#diagnostics').value = data.diagnostics;
  if (data.shortcutError) status.textContent = `Shortcut unavailable: ${data.shortcutError}`;
}
document.querySelector('form').addEventListener('submit', async (event) => {
  event.preventDefault();
  try {
    const settings = { ...current };
    for (const key of Object.keys(fields)) settings[key] = document.getElementById(key).checked;
    settings.shortcut = document.querySelector('#shortcut').value.trim() || null;
    settings.zoom = Number(document.querySelector('#zoom').value);
    await invoke('save_preferences', { settings }); await load(); status.textContent = 'Preferences saved.';
  } catch (error) { status.textContent = String(error); }
});
document.querySelector('#reset').addEventListener('click', async () => {
  if (!window.confirm('Reset app preferences? Your Google session will be kept.')) return;
  try { await invoke('save_preferences', { settings: null }); await load(); status.textContent = 'Preferences reset.'; }
  catch (error) { status.textContent = String(error); }
});
load().catch((error) => { status.textContent = String(error); });
