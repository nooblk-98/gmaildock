(() => {
  if (window !== window.top) return;

  const accountSelector = 'a[href*="myaccount.google.com"]';
  const appsSelector =
    'a[href*="about/products"], [aria-label="Google apps"], [title="Google apps"]';
  const interactive = 'a, button, [role="button"]';

  function hide(element) {
    const target = element.closest(interactive) || element;
    target.style.setProperty('display', 'none', 'important');
  }

  function scan(root) {
    if (!(root instanceof Element)) return;
    if (root.matches(accountSelector) || root.matches(appsSelector)) hide(root);
    for (const element of root.querySelectorAll(`${accountSelector}, ${appsSelector}`)) hide(element);
  }

  function mount() {
    if (!document.body) return;

    scan(document.documentElement);
    new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) scan(node);
      }
    }).observe(document.body, { childList: true, subtree: true });

    if (location.hostname !== 'mail.google.com') return;

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
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount, { once: true });
  } else {
    mount();
  }
})();
