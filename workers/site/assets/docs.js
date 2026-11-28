(() => {
  const search = document.querySelector('.doc-search');
  const input = search.querySelector('input');
  const results = search.querySelector('.doc-search-results');
  const count = search.querySelector('.doc-search-status');
  const list = search.querySelector('.doc-search-list');
  const zh = search.dataset.locale === 'zh';
  let index;

  function loadIndex() {
    if (!index) {
      index = fetch('/assets/docs-search.json').then(response => {
        if (!response.ok) throw new Error('Search index unavailable');
        return response.json();
      }).catch(error => {
        index = undefined;
        throw error;
      });
    }
    return index;
  }

  async function findGuides() {
    const query = input.value.trim().toLowerCase();
    results.hidden = !query;
    list.replaceChildren();
    if (!query) return;
    count.textContent = zh ? '正在搜索…' : 'Searching…';
    let guides;
    try {
      guides = await loadIndex();
    } catch {
      if (input.value.trim().toLowerCase() === query) {
        count.textContent = zh ? '暂时无法搜索。可以从文档目录浏览指南。' : 'Search is unavailable. Browse the documentation directory.';
      }
      return;
    }
    if (input.value.trim().toLowerCase() !== query) return;
    const terms = query.split(/\s+/);
    const found = [];
    for (const guide of guides) {
      const title = (guide.title + ' ' + guide.en + ' ' + guide.zh).toLowerCase();
      const searchable = (title + ' ' + guide.group + ' ' + guide.groupZh + ' ' + guide.text).toLowerCase();
      if (terms.every(term => searchable.includes(term))) {
        found.push({ guide, rank: terms.filter(term => title.includes(term)).length });
      }
    }
    found.sort((a, b) => b.rank - a.rank);
    count.textContent = found.length
      ? (zh ? '找到 ' + found.length + ' 篇指南' : found.length + (found.length === 1 ? ' guide' : ' guides'))
      : (zh ? '未找到匹配的指南。' : 'No matching guides.');
    for (const { guide } of found) {
      const item = document.createElement('li');
      const link = document.createElement('a');
      link.href = (zh ? '/zh/docs/' : '/docs/') + guide.id + '/';
      link.textContent = zh ? guide.zh : guide.en;
      const context = document.createElement('small');
      const at = guide.text.toLowerCase().indexOf(terms[0]);
      if (at >= 0) {
        const start = Math.max(0, at - 25);
        context.lang = 'en';
        context.textContent = (start ? '…' : '') + guide.text.slice(start, start + 120) + (start + 120 < guide.text.length ? '…' : '');
      } else {
        context.textContent = zh ? guide.groupZh : guide.group;
      }
      item.append(link, context);
      list.append(item);
    }
  }

  input.addEventListener('input', findGuides);
  search.hidden = false;

  const contents = document.querySelector('.doc-toc details');
  if (contents) {
    const wide = matchMedia('(min-width: 44.01rem)');
    contents.open = wide.matches;
    wide.addEventListener('change', () => { contents.open = wide.matches; });
  }

  const status = document.querySelector('.doc-copy-status');
  for (const pre of document.querySelectorAll('.doc-reading pre')) {
    const block = document.createElement('div');
    block.className = 'doc-code-block';
    const tools = document.createElement('div');
    tools.className = 'doc-code-tools';
    tools.lang = zh ? 'zh' : 'en';
    const language = document.createElement('span');
    const code = pre.querySelector('code');
    const type = code && [...code.classList].find(name => name.startsWith('language-'));
    language.textContent = type ? type.slice('language-'.length) : '';
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = zh ? '复制代码' : 'Copy code';
    button.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(pre.textContent);
        button.textContent = zh ? '已复制' : 'Copied';
        status.textContent = zh ? '代码已复制。' : 'Code copied.';
        setTimeout(() => { button.textContent = zh ? '复制代码' : 'Copy code'; }, 1500);
      } catch {
        const range = document.createRange();
        range.selectNodeContents(pre);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        button.textContent = zh ? '已选中代码' : 'Code selected';
        status.textContent = zh ? '代码已选中，请手动复制。' : 'Code selected. Copy the selected text manually.';
      }
    });
    tools.append(language, button);
    pre.before(block);
    block.append(tools, pre);
  }
})();
