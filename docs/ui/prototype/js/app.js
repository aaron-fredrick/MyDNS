/* Shared prototype bootstrap. Backend integration belongs to the future React/Rust implementation. */
document.documentElement.dataset.theme = localStorage.getItem('mydns-theme') || 'dark';
window.MyDNSPrototype = window.MyDNSPrototype || {};
window.MyDNSPrototype.setTheme = function (theme) {
  document.documentElement.dataset.theme = theme;
  localStorage.setItem('mydns-theme', theme);
};
