"""Verify public document routes and runnable examples against the offline guides."""
import base64
import json
import tempfile
import unittest
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import parse_qs, unquote, urljoin, urlsplit

from docsite import write_docs
from tools.docs.build import OUT, compile_pages


class DocumentLinks(HTMLParser):
    def __init__(self, markup):
        super().__init__()
        self.anchors = set()
        self.links = []
        self.tries = []
        self.feed(markup)

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if 'id' in attributes:
            self.anchors.add(attributes['id'])
        if tag == 'a' and 'href' in attributes:
            self.links.append(attributes['href'])
            if 'doc-try' in attributes.get('class', '').split():
                self.tries.append(attributes['href'])


class PublicDocsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.tmp.cleanup)
        cls.out = Path(cls.tmp.name)
        write_docs(cls.out)
        cls.guides = compile_pages()
        cls.pages = {}
        for file in cls.out.rglob('index.html'):
            route = '/' + file.parent.relative_to(cls.out).as_posix() + '/'
            cls.pages[route] = DocumentLinks(file.read_text(encoding='utf-8'))

    def test_article_routes_and_fragments(self):
        for prefix in ('/docs/', '/zh/docs/'):
            self.assertIn(prefix, self.pages)
            for guide in self.guides:
                route = prefix + guide['id'] + '/'
                document = self.pages[route]
                for _, anchor, _ in guide['headings']:
                    self.assertIn(anchor, document.anchors)
                for href in document.links:
                    target = urlsplit(urljoin(route, href))
                    if target.netloc or not target.path.startswith(('/docs/', '/zh/docs/')):
                        continue
                    self.assertIn(target.path, self.pages, (route, href))
                    if target.fragment:
                        self.assertIn(unquote(target.fragment), self.pages[target.path].anchors, (route, href))

    def test_examples_open_with_original_source_and_geometry(self):
        for guide in self.guides:
            for prefix in ('/docs/', '/zh/docs/'):
                links = self.pages[prefix + guide['id'] + '/'].tries
                self.assertEqual(len(links), len(guide['examples']))
                for url, example in zip(links, guide['examples']):
                    target = urlsplit(url)
                    self.assertEqual(target.path, '/playground/')
                    self.assertFalse(target.query)
                    settings = dict((k, v[0]) for k, v in parse_qs(target.fragment).items())
                    self.assertEqual(settings['f'], example['language'])
                    self.assertEqual(settings['u'], 'mm')
                    self.assertAlmostEqual(float(settings['w']) * 25.4, example['width'])
                    self.assertAlmostEqual(float(settings['h']) * 25.4, example['height'])
                    self.assertEqual(int(settings['d']), example['dpmm'])
                    self.assertTrue(settings['c'].startswith('b'))
                    source = base64.urlsafe_b64decode(settings['c'][1:] + '===')
                    self.assertEqual(source, example['source'].encode())

    def test_index_covers_current_offline_guides(self):
        index = json.loads((self.out / 'assets/docs-search.json').read_text(encoding='utf-8'))
        embedded = OUT.read_text(encoding='utf-8').split('application/json">', 1)[1].split('</script>', 1)[0]
        self.assertEqual([g['id'] for g in index], [g['id'] for g in json.loads(embedded)])
        for guide in index:
            self.assertTrue(guide['en'] and guide['zh'] and guide['text'])


if __name__ == '__main__':
    unittest.main()
