const menuToggle = document.querySelector(".site-nav-toggle");
const navigation = document.querySelector(".site-nav");
const compact = window.matchMedia("(max-width: 46rem)");

function showMenu(open) {
  navigation.hidden = !open && compact.matches;
  menuToggle.setAttribute("aria-expanded", String(open));
}

function sizeNavigation() {
  menuToggle.hidden = !compact.matches;
  showMenu(false);
}

menuToggle.addEventListener("click", () => showMenu(navigation.hidden));
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && compact.matches && !navigation.hidden) {
    showMenu(false);
    menuToggle.focus();
  }
});
document.addEventListener("click", (event) => {
  if (compact.matches && !navigation.hidden && !event.target.closest(".site-bar")) showMenu(false);
});
compact.addEventListener("change", sizeNavigation);
sizeNavigation();

document.querySelector("[data-site-locale]").addEventListener("click", (event) => {
  const link = event.currentTarget;
  try { localStorage.setItem("labelize-lang", link.dataset.siteLocale); } catch {}
  // The two versions share article headings and home section IDs.
  link.hash = location.hash;
});
