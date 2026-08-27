/* Prototype-only navigation helpers. */
document.addEventListener('click', (event) => {
  const link = event.target.closest('[data-nav]');
  if (!link) return;
  document.querySelectorAll('[data-nav]').forEach((item) => item.classList.remove('active'));
  link.classList.add('active');
});
