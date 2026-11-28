// Old public links used URL fragments, which the browser never sends to the Worker.
(() => {
  const hash = location.hash;
  const guide = /^#docs(?:\/([a-z0-9-]+)(?:\/(.+))?)?\/?$/i.exec(hash);
  let target;
  if (guide) {
    const path = guide[1] ? "/docs/" + guide[1] + "/" : "/docs/";
    target = path + location.search + (guide[2] ? "#" + guide[2] : "");
  } else if (hash === "#playground" || hash === "#api" || new URLSearchParams(hash.slice(1)).has("c")) {
    target = "/playground/" + location.search + hash;
  }
  if (target) location.replace(target);
})();
