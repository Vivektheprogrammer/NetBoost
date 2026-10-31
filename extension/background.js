// NetBoost Chrome / Edge Extension Background Service Worker
const NETBOOST_IPC_URL = 'http://127.0.0.1:52140/intercept';
const NETBOOST_STATUS_URL = 'http://127.0.0.1:52140/status';

const DOWNLOAD_EXTENSIONS = [
  '.zip', '.rar', '.7z', '.tar', '.gz', '.bz2', '.xz',
  '.iso', '.img', '.dmg', '.vhd',
  '.exe', '.msi', '.bin', '.dat', '.apk',
  '.mp4', '.mkv', '.avi', '.mov', '.flv',
  '.pdf', '.epub'
];

let bypassNextDownload = false;

// Listen for Alt-click bypass messages from content script
chrome.runtime.onMessage.addListener((message) => {
  if (message.type === 'BYPASS_NETBOOST') {
    bypassNextDownload = true;
    setTimeout(() => { bypassNextDownload = false; }, 3000);
  }
});

// Check if NetBoost desktop application is active
async function isNetBoostRunning() {
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 600);
    const resp = await fetch(NETBOOST_STATUS_URL, { signal: controller.signal });
    clearTimeout(timeoutId);
    return resp.ok;
  } catch (e) {
    return false;
  }
}

// Track download states for diagnostics
chrome.downloads.onChanged.addListener((delta) => {
  if (delta.state) {
    console.log(`[NetBoost] Download #${delta.id} state changed:`, delta.state.current);
  }
  if (delta.error) {
    console.warn(`[NetBoost] Download #${delta.id} error:`, delta.error.current);
  }
});

// Intercept browser downloads cleanly without ever blocking Chrome's native "Save As" dialog
chrome.downloads.onCreated.addListener((downloadItem) => {
  // Synchronously return undefined so Chrome NEVER blocks, delays, or suspends the download pipeline!
  handleDownloadCreated(downloadItem);
});

async function handleDownloadCreated(downloadItem) {
  // If user held Alt key or disabled capture, download normally in browser
  if (bypassNextDownload) {
    bypassNextDownload = false;
    return;
  }

  // Ignore data/blob/empty URLs
  if (!downloadItem.url || downloadItem.url.startsWith('data:') || downloadItem.url.startsWith('blob:')) {
    return;
  }

  // Check if interceptor is enabled
  const config = await chrome.storage.local.get({ enabled: true });
  if (!config.enabled) return;

  const urlLower = downloadItem.url.toLowerCase();
  const filename = downloadItem.filename ? downloadItem.filename.split(/[\\/]/).pop() : '';
  const fileLower = filename.toLowerCase();

  const matchesExt = DOWNLOAD_EXTENSIONS.some(ext => fileLower.endsWith(ext) || urlLower.includes(ext));
  const isBinaryMime = downloadItem.mime && (
    downloadItem.mime.includes('octet-stream') || 
    downloadItem.mime.includes('zip') || 
    downloadItem.mime.includes('tar') || 
    downloadItem.mime.includes('iso') ||
    downloadItem.mime.includes('x-msdos-program')
  );

  if (!matchesExt && !isBinaryMime) {
    // Normal document/web page, let browser handle it
    return;
  }

  // Check if NetBoost desktop engine is running
  const running = await isNetBoostRunning();
  if (!running) {
    return;
  }

  console.log('[NetBoost] Candidate download detected:', downloadItem.id, downloadItem.url);

  // Gather cookies for the target domain
  let cookieString = '';
  try {
    const cookies = await chrome.cookies.getAll({ url: downloadItem.url });
    if (cookies && cookies.length > 0) {
      cookieString = cookies.map(c => `${c.name}=${c.value}`).join('; ');
    }
  } catch (e) {
    console.warn('[NetBoost] Cookie retrieval failed:', e);
  }

  // Forward to NetBoost local server and wait for user's decision
  const payload = {
    id: downloadItem.id,
    url: downloadItem.url,
    filename: filename || undefined,
    referrer: downloadItem.referrer || undefined,
    cookies: cookieString || undefined,
    source: 'browser_extension'
  };

  let shouldAccelerate = false;
  try {
    const resp = await fetch(NETBOOST_IPC_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    if (resp.ok) {
      const data = await resp.json();
      shouldAccelerate = data.accelerate === true;
    }
  } catch (err) {
    console.warn('[NetBoost] NetBoost IPC response error, continuing in browser:', err);
    shouldAccelerate = false;
  }

  // Resolve download based on user choice:
  if (shouldAccelerate) {
    console.log('[NetBoost] Accelerating download via NetBoost engine, canceling browser copy:', downloadItem.id);
    try {
      await chrome.downloads.cancel(downloadItem.id);
      await chrome.downloads.erase({ id: downloadItem.id });
    } catch (e) {}
  } else {
    console.log('[NetBoost] User selected browser download. Leaving Chrome download intact:', downloadItem.id);
  }
}
