/* Mock interactions only: no DNS/API/WebSocket implementation. */
document.addEventListener('click', (event) => {
  const theme = event.target.closest('[data-theme-toggle]');
  if (theme) {
    const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    window.MyDNSPrototype?.setTheme(next);
  }
});
