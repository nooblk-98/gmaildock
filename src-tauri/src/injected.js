(() => {
  const accountLink = 'Manage your Google Account';
  const interactive = 'a, button, [role="button"]';

  function hideAccountLink(element) {
    if (element.textContent?.trim() === accountLink) {
      element.style.setProperty('display', 'none', 'important');
    }
  }

  function scan(root) {
    if (root.nodeType === Node.TEXT_NODE) {
      const parent = root.parentElement?.closest(interactive);
      if (parent) hideAccountLink(parent);
      return;
    }
    if (root.nodeType !== Node.ELEMENT_NODE) return;
    if (root.matches(interactive)) hideAccountLink(root);
    for (const element of root.querySelectorAll(interactive)) hideAccountLink(element);
  }

  function mount() {
    scan(document.documentElement);
    new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) scan(node);
      }
    }).observe(document.documentElement, { childList: true, subtree: true });

    if (window !== window.top || location.hostname !== 'mail.google.com') return;

    let button;
    let availableVersion;
    let checking = false;

    function showUpdate(version) {
      if (!button) {
        button = document.createElement('button');
        button.id = 'rustgmail-update-button';
        button.type = 'button';
        Object.assign(button.style, {
          position: 'fixed', right: '18px', bottom: '18px', zIndex: '2147483647',
          border: '0', borderRadius: '999px', padding: '10px 15px',
          background: '#d6f4e9', color: '#263238', boxShadow: '0 3px 14px #0002',
          font: '600 12px system-ui, sans-serif', cursor: 'pointer',
        });
        button.addEventListener('click', async () => {
          if (!availableVersion || !confirm(`Install GmailDock v${availableVersion} now?`)) return;
          button.disabled = true;
          button.textContent = 'Installing update…';
          try {
            await window.__TAURI__.core.invoke('install_update');
          } catch (error) {
            button.disabled = false;
            button.textContent = 'Update failed — retry';
            button.title = String(error);
          }
        });
        document.body.append(button);
      }
      button.textContent = `Update to v${version}`;
      button.title = `Install GmailDock v${version}`;
    }

    async function check() {
      if (checking || button?.disabled || !window.__TAURI__?.core) return;
      checking = true;
      try {
        const version = await window.__TAURI__.core.invoke('check_for_updates');
        availableVersion = version;
        if (version) showUpdate(version);
        else if (button) {
          button.remove();
          button = undefined;
        }
      } catch {
        // A failed background check leaves the current button state intact.
      } finally {
        checking = false;
      }
    }

    setTimeout(check, 8000);
    setInterval(check, 30 * 60 * 1000);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount, { once: true });
  } else {
    mount();
  }
})();
