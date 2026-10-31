// NetBoost Content Script - Alt-Click bypass detection
window.addEventListener('click', (e) => {
  // If user holds the Alt key while clicking any download link, bypass NetBoost!
  if (e.altKey) {
    try {
      chrome.runtime.sendMessage({ type: 'BYPASS_NETBOOST' });
    } catch (err) {}
  }
}, true);

