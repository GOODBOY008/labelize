# Studio printer template interoperability fixtures

Each `printers/shipping.tpl.*` file is a Studio export for 100 × 50 mm media
at 8 dots/mm. The DPL resource is Studio’s `shipping-logo.tpl.dpl`, which
downloads a binary, one-bit BMP before printing the variable fields.
The two JSON records supply the same order number to visible text, Code 128
and QR. They exercise leading zeros, quotes, a backslash, printer command
delimiters, and literal double braces in the product description.

The templates come from Labelize Studio’s [MIT-licensed examples](LICENSE-Studio).
The ZPL file matches the published shipping template at Studio commit
`15afc493`. The other four exports came from its `codex/multilanguage-templates`
development checkout based on that revision. The ten expected printer jobs
were captured from the Studio Java filler during initial compatibility testing.
[Studio source revision](https://github.com/GOODBOY008/labelize-studio/tree/15afc4930cca83145a8a9cca88aa1746c74f7869)

`utf8.tpl.tspl` and `utf8.tpl.cpcl` were exported at 8 dots/mm with Studio’s
`labelize-printer-daemon export-template`. They use the same shipping design
with `version` set to 4 and `samples` set to [the Chinese order](utf8.json).
The `printer_profiles` entries select `encoding: "utf-8"` and fonts
`TSS24.BF2` for TSPL and `GBUNSG24.CPF` for CPCL. Their acceptance test checks
the native UTF-8 bytes, the three distinct printed glyphs for 保温杯, and the
searchable product name in a vector PDF.

## Verify the fixtures

`printers/first.*` and `printers/second.*` are the expected native printer jobs.
Rust tests fill one loaded template with both records and compare every byte
against those files. The CLI tests render each job at 8 and
12 dots/mm, compare PNG and vector PDF output with ordinary conversion, extract
the visible text, and independently scan Code 128 and QR from the PNGs.

```sh
cargo test --locked --features cli,vector --test e2e_templates
```

For downstream integration examples, see the [printer template guide](../../docs/PRINT_TEMPLATES.md).
