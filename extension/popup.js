const NETBOOST_STATUS_URL = 'http://127.0.0.1:52140/status';

document.addEventListener('DOMContentLoaded', async () => {
  const dot = document.getElementById('dot');
  const label = document.getElementById('status-label');
  const toggle = document.getElementById('enabled-toggle');

  // Load toggle state
  const config = await chrome.storage.local.get({ enabled: true });
  toggle.checked = config.enabled;

  toggle.addEventListener('change', () => {
    chrome.storage.local.set({ enabled: toggle.checked });
  });

  // Check NetBoost connection
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 1000);
    const resp = await fetch(NETBOOST_STATUS_URL, { signal: controller.signal });
    clearTimeout(timeout);

    if (resp.ok) {
      dot.className = 'indicator online';
      label.textContent = 'Connected';
      label.style.color = '#10b981';
    } else {
      throw new Error();
    }
  } catch (e) {
    dot.className = 'indicator offline';
    label.textContent = 'Not Running';
    label.style.color = '#ef4444';
  }
});

