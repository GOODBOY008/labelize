const zh = document.documentElement.lang.startsWith("zh");
const demos = JSON.parse(document.getElementById("demo-content").textContent);
const languageGroup = document.querySelector(".language-options");

languageGroup.addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (!button) return;
  const language = button.dataset.language;
  const demo = demos[language];
  document.getElementById("demo-image").src = demo.image;
  document.getElementById("demo-file").textContent = "order." + language;
  document.getElementById("demo-source").textContent = demo.source;
  document.getElementById("demo-open").href = demo.url;
  for (const option of languageGroup.querySelectorAll("button")) {
    option.setAttribute("aria-pressed", String(option === button));
  }
});
languageGroup.hidden = false;

const choices = [...document.querySelectorAll("[data-choice]")];
const panels = [...document.querySelectorAll("[data-panel]")];
function selectIntegration(name) {
  for (const panel of panels) panel.hidden = panel.dataset.panel !== name;
  for (const choice of choices) {
    if (choice.dataset.choice === name) choice.setAttribute("aria-current", "true");
    else choice.removeAttribute("aria-current");
  }
}
for (const choice of choices) {
  choice.addEventListener("click", (event) => {
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    selectIntegration(choice.dataset.choice);
  });
}

function chooseFromHash() {
  const selected = panels.find((panel) => "#" + panel.id === location.hash);
  selectIntegration(selected?.dataset.panel ?? "cli");
}
chooseFromHash();
window.addEventListener("hashchange", chooseFromHash);
if (navigator.clipboard?.writeText) {
  for (const button of document.querySelectorAll(".copy-example")) {
    button.hidden = false;
    button.addEventListener("click", async () => {
      const sample = button.closest(".example-panel").querySelector("pre code");
      try {
        await navigator.clipboard.writeText(sample.textContent);
        button.querySelector("span").textContent = zh ? "已复制" : "Copied";
        setTimeout(() => { button.querySelector("span").textContent = zh ? "复制" : "Copy"; }, 1600);
      } catch {
        const range = document.createRange();
        range.selectNodeContents(sample);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        sample.parentElement.focus();
      }
    });
  }
}
