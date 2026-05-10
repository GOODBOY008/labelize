# Split ZPL Files by Type Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Separate ZPL files into `testdata/labels/` (real carrier labels) and `testdata/unit/` (synthetic test fixtures) so they are no longer mixed in a flat directory.

**Architecture:** Move files into two new subdirectories, add helper functions in `render_helpers.rs` to resolve paths per category, and update all test files to use the new helpers. The `testdata/` root keeps non-ZPL files (`dpduk.epl`, `dpduk.clp`, `dpduk.html`) and orphan PNGs (`dpduk.png`, `pnldpd_page_2.png`, `ups_inverted.png`).

**Tech Stack:** Rust, cargo test

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `tests/common/render_helpers.rs` | Add `labels_dir()` and `unit_dir()` helpers |
| Modify | `tests/e2e_golden.rs` | Update path construction to use subdirectory helpers |
| Modify | `tests/e2e_diff_report.rs` | Scan both subdirectories instead of root |
| Modify | `tests/unit_property_tests.rs` | Update `text_orientation_tests` and `preservation_tests` paths |
| Modify | `tests/unit_skill.rs` | Update hardcoded ZPL paths |
| Modify | `tests/unit_golden_png_dimensions.rs` | Update PNG discovery to scan subdirectories |
| Move | `testdata/*.zpl` + matching `*.png` | Into `testdata/labels/` or `testdata/unit/` |

---

## File Categorization

### Real labels -> `testdata/labels/` (42 ZPL + 42 PNG pairs)

`amazon`, `amazonshipping`, `auspost`, `bpost`, `brtit`, `bstc`, `canadapost`, `colissimo`, `correos`, `dbs`, `dbschenker`, `dhlecommercetr`, `dhlpaket`, `dhlparcelit`, `dhlparceluk`, `dpdde`, `dpdpl`, `evri`, `fedex`, `glscz`, `glsdk_return`, `icapaket`, `inpost`, `jcpenney`, `kmart`, `labelary`, `ontrac`, `pnldpd`, `pocztex`, `porterbuddy`, `posteit`, `posten`, `postnl`, `purolator`, `royalmail`, `seur`, `swisspost`, `tnt_express`, `ups`, `ups_surepost`, `usps`, `yodel`

### Unit test fixtures -> `testdata/unit/` (40 ZPL + 40 PNG pairs)

`aztec_ec`, `barcode128_default_width`, `barcode128_line`, `barcode128_line_above`, `barcode128_mode_a`, `barcode128_mode_d`, `barcode128_mode_n`, `barcode128_mode_n_cba_sets`, `barcode128_mode_u`, `barcode128_rotated`, `ean13`, `edi_triangle`, `encodings_013`, `gb_0_height`, `gb_0_width`, `gb_normal`, `gb_rounded`, `gd_default_params`, `gd_thick`, `gd_thin_l`, `gd_thin_r`, `gs`, `pdf417_basic`, `qr_code_ft_manual`, `qr_code_offset`, `return_qrcode`, `reverse`, `reverse_qr`, `templating`, `text_fallback_default`, `text_fo_b`, `text_fo_i`, `text_fo_n`, `text_fo_r`, `text_ft_auto_pos`, `text_ft_b`, `text_ft_i`, `text_ft_n`, `text_ft_r`, `text_multiline`

### Stays at `testdata/` root

- `dpduk.epl`, `dpduk.clp`, `dpduk.html` (non-ZPL files)
- `dpduk.png` (EPL golden, no matching .zpl)
- `pnldpd_page_2.png`, `ups_inverted.png` (orphan PNGs with no matching .zpl)

---

### Task 1: Create subdirectories and move files

**Files:**
- Create: `testdata/labels/` (directory)
- Create: `testdata/unit/` (directory)
- Move: 42 ZPL+PNG pairs into `testdata/labels/`
- Move: 40 ZPL+PNG pairs into `testdata/unit/`

- [ ] **Step 1: Create the two subdirectories**

```bash
mkdir -p testdata/labels testdata/unit
```

- [ ] **Step 2: Move real label files to `testdata/labels/`**

```bash
cd /Volumes/AidenExternal/aiden/IdeaProjects/labelize
for name in amazon amazonshipping auspost bpost brtit bstc canadapost colissimo correos dbs dbschenker dhlecommercetr dhlpaket dhlparcelit dhlparceluk dpdde dpdpl evri fedex glscz glsdk_return icapaket inpost jcpenney kmart labelary ontrac pnldpd pocztex porterbuddy posteit posten postnl purolator royalmail seur swisspost tnt_express ups ups_surepost usps yodel; do
  mv "testdata/${name}.zpl" "testdata/labels/" 2>/dev/null
  mv "testdata/${name}.png" "testdata/labels/" 2>/dev/null
done
```

- [ ] **Step 3: Move unit test files to `testdata/unit/`**

```bash
for name in aztec_ec barcode128_default_width barcode128_line barcode128_line_above barcode128_mode_a barcode128_mode_d barcode128_mode_n barcode128_mode_n_cba_sets barcode128_mode_u barcode128_rotated ean13 edi_triangle encodings_013 gb_0_height gb_0_width gb_normal gb_rounded gd_default_params gd_thick gd_thin_l gd_thin_r gs pdf417_basic qr_code_ft_manual qr_code_offset return_qrcode reverse reverse_qr templating text_fallback_default text_fo_b text_fo_i text_fo_n text_fo_r text_ft_auto_pos text_ft_b text_ft_i text_ft_n text_ft_r text_multiline; do
  mv "testdata/${name}.zpl" "testdata/unit/" 2>/dev/null
  mv "testdata/${name}.png" "testdata/unit/" 2>/dev/null
done
```

- [ ] **Step 4: Verify only non-ZPL/orphan files remain at root**

```bash
ls testdata/*.zpl 2>/dev/null  # should be empty
ls testdata/*.png 2>/dev/null  # should show only dpduk.png, pnldpd_page_2.png, ups_inverted.png
ls testdata/*.epl testdata/*.clp testdata/*.html 2>/dev/null  # dpduk files
```

Expected: No `.zpl` files remain at `testdata/` root. Only `dpduk.png`, `pnldpd_page_2.png`, `ups_inverted.png`, and the `dpduk.{epl,clp,html}` files remain.

- [ ] **Step 5: Verify file counts**

```bash
ls testdata/labels/*.zpl | wc -l  # expect 42
ls testdata/labels/*.png | wc -l  # expect 42
ls testdata/unit/*.zpl | wc -l   # expect 40
ls testdata/unit/*.png | wc -l   # expect 40
```

- [ ] **Step 6: Commit**

```bash
git add testdata/labels/ testdata/unit/
git commit -m "refactor: split ZPL files into labels/ and unit/ subdirectories"
```

---

### Task 2: Add subdirectory helpers to `render_helpers.rs`

**Files:**
- Modify: `tests/common/render_helpers.rs:54-64`

- [ ] **Step 1: Add `labels_dir()` and `unit_dir()` helpers**

Add these two functions right after the existing `testdata_dir()` function (after line 64):

```rust
/// Returns the path to the `testdata/labels/` directory (real carrier labels).
pub fn labels_dir() -> std::path::PathBuf {
    testdata_dir().join("labels")
}

/// Returns the path to the `testdata/unit/` directory (synthetic test fixtures).
pub fn unit_dir() -> std::path::PathBuf {
    testdata_dir().join("unit")
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check --tests 2>&1 | tail -5
```

Expected: no errors (new functions aren't called yet, but they compile).

- [ ] **Step 3: Commit**

```bash
git add tests/common/render_helpers.rs
git commit -m "feat: add labels_dir() and unit_dir() helpers to render_helpers"
```

---

### Task 3: Update `e2e_golden.rs` to use subdirectory paths

**Files:**
- Modify: `tests/e2e_golden.rs`

The file has ~75 golden test functions. Each calls `golden_zpl_with_tolerance(name, tol)` or `golden_epl_with_tolerance(name, tol)`. The helper functions construct paths using `testdata_dir().join(format!("{}.zpl", name))`.

We need to change the helper functions so they try `labels/` first, then `unit/`, then fall back to root (for `dpduk.epl`).

- [ ] **Step 1: Update `golden_zpl_with_tolerance` to search subdirectories**

Find the `golden_zpl_with_tolerance` function. Replace the path construction logic. The function currently does:

```rust
fn golden_zpl_with_tolerance(name: &str, tolerance: f64) {
    let dir = testdata_dir();
    let input = dir.join(format!("{}.zpl", name));
    let expected = dir.join(format!("{}.png", name));
```

Change it to:

```rust
fn golden_zpl_with_tolerance(name: &str, tolerance: f64) {
    let dir = testdata_dir();
    // Try labels/ first, then unit/, then root
    let input = if dir.join("labels").join(format!("{}.zpl", name)).exists() {
        dir.join("labels").join(format!("{}.zpl", name))
    } else if dir.join("unit").join(format!("{}.zpl", name)).exists() {
        dir.join("unit").join(format!("{}.zpl", name))
    } else {
        dir.join(format!("{}.zpl", name))
    };
    let expected = input.with_extension("png");
```

- [ ] **Step 2: Update `golden_epl_with_tolerance` similarly**

Find the `golden_epl_with_tolerance` function. The EPL file `dpduk` stays at root, so no subdirectory search needed -- but verify the path still works. The function does:

```rust
fn golden_epl_with_tolerance(name: &str, tolerance: f64) {
    let dir = testdata_dir();
    let input = dir.join(format!("{}.epl", name));
    let expected = dir.join(format!("{}.png", name));
```

No change needed for EPL since `dpduk.epl` and `dpduk.png` stay at root.

- [ ] **Step 3: Run golden tests to verify**

```bash
cargo test --test e2e_golden -- --test-threads=4 2>&1 | tail -20
```

Expected: all tests pass (or SKIP if they were already skipping). No new failures.

- [ ] **Step 4: Commit**

```bash
git add tests/e2e_golden.rs
git commit -m "refactor: update e2e_golden to resolve ZPL from labels/ and unit/ subdirs"
```

---

### Task 4: Update `e2e_diff_report.rs` to scan subdirectories

**Files:**
- Modify: `tests/e2e_diff_report.rs`

The `generate_diff_report()` function uses `read_dir` on `testdata/` to discover `.zpl`/`.epl` files. After the move, no `.zpl` files remain at root -- only `.epl`. We need to scan `labels/` and `unit/` as well.

- [ ] **Step 1: Update `generate_diff_report` to scan all three locations**

Find the `generate_diff_report` function. The current discovery logic is:

```rust
let dir = render_helpers::testdata_dir();
let mut label_files: Vec<_> = std::fs::read_dir(&dir)
    .expect("read testdata dir")
    .flatten()
    .filter(|e| {
        let ext = e.path().extension().map(|x| x.to_string_lossy().to_string());
        matches!(ext.as_deref(), Some("zpl") | Some("epl"))
    })
    .map(|e| e.path())
    .collect();
```

Replace it with:

```rust
let dir = render_helpers::testdata_dir();
let scan_dirs = [
    dir.clone(),
    dir.join("labels"),
    dir.join("unit"),
];
let mut label_files: Vec<_> = scan_dirs
    .iter()
    .flat_map(|d| {
        std::fs::read_dir(d).into_iter().flatten().flatten()
    })
    .filter(|e| {
        let ext = e.path().extension().map(|x| x.to_string_lossy().to_string());
        matches!(ext.as_deref(), Some("zpl") | Some("epl"))
    })
    .map(|e| e.path())
    .collect();
```

- [ ] **Step 2: Update the reference PNG lookup**

The current code looks for the reference PNG in the same directory as the ZPL file:

```rust
let ref_png = dir.join(format!("{}.png", name));
```

Since ZPL and PNG are now co-located in the same subdirectory, change this to look relative to the ZPL file's parent:

```rust
let ref_png = path.parent().unwrap().join(format!("{}.png", name));
```

- [ ] **Step 3: Run the diff report test**

```bash
cargo test --test e2e_diff_report -- --test-threads=1 2>&1 | tail -20
```

Expected: test passes, diff report is generated.

- [ ] **Step 4: Commit**

```bash
git add tests/e2e_diff_report.rs
git commit -m "refactor: update diff report to scan labels/ and unit/ subdirs"
```

---

### Task 5: Update `unit_property_tests.rs`

**Files:**
- Modify: `tests/unit_property_tests.rs`

Two submodules reference ZPL files from testdata:
- `text_orientation_tests` (lines 126-211): uses names `text_fo_n`, `text_fo_r`, `text_fo_i`, `text_fo_b`, `text_ft_n`, `text_ft_r`, `text_ft_i`, `text_ft_b`, `text_ft_auto_pos`, `text_multiline` -- all unit fixtures
- `preservation_tests` (lines 217-321): uses names `barcode128_default_width`, `barcode128_rotated`, `barcode128_line`, `ean13`, `gb_normal`, `gb_rounded`, `gb_0_height`, `gb_0_width`, `amazon`, `fedex`, `ups`, `usps`, `qr_code_ft_manual`, `reverse_qr` -- mix of labels and unit

- [ ] **Step 1: Update `run_text_golden` to use `unit_dir()`**

Find the `run_text_golden` function. Replace:

```rust
fn run_text_golden(name: &str, tolerance: f64) {
    let dir = render_helpers::testdata_dir();
    let input = dir.join(format!("{}.zpl", name));
    let expected = dir.join(format!("{}.png", name));
```

With:

```rust
fn run_text_golden(name: &str, tolerance: f64) {
    let dir = render_helpers::unit_dir();
    let input = dir.join(format!("{}.zpl", name));
    let expected = dir.join(format!("{}.png", name));
```

- [ ] **Step 2: Update `run_preservation_golden` to search subdirectories**

Find the `run_preservation_golden` function. Replace:

```rust
fn run_preservation_golden(name: &str) {
    let dir = render_helpers::testdata_dir();
    let input = dir.join(format!("{}.zpl", name));
    let expected = dir.join(format!("{}.png", name));
```

With:

```rust
fn run_preservation_golden(name: &str) {
    let dir = render_helpers::testdata_dir();
    // Try labels/ first, then unit/, then root
    let input = if dir.join("labels").join(format!("{}.zpl", name)).exists() {
        dir.join("labels").join(format!("{}.zpl", name))
    } else if dir.join("unit").join(format!("{}.zpl", name)).exists() {
        dir.join("unit").join(format!("{}.zpl", name))
    } else {
        dir.join(format!("{}.zpl", name))
    };
    let expected = input.with_extension("png");
```

- [ ] **Step 3: Run property tests**

```bash
cargo test --test unit_property_tests -- --test-threads=4 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/unit_property_tests.rs
git commit -m "refactor: update property tests to use labels/ and unit/ subdirs"
```

---

### Task 6: Update `unit_skill.rs`

**Files:**
- Modify: `tests/unit_skill.rs`

This file references ZPL files in two ways:
1. Hardcoded: `testdata.join("bstc.zpl")`, `testdata.join("amazon.zpl")` -- both real labels
2. Dynamic via `diff_scanner::load_diff_report` -- reads from `testdata/diffs/diff_report.txt`

- [ ] **Step 1: Update hardcoded paths to use `labels_dir()`**

Find and replace the two hardcoded path constructions. The `bstc` and `amazon` references:

```rust
let testdata = render_helpers::testdata_dir();
// ...
let zpl_path = testdata.join("bstc.zpl");
```

Change to:

```rust
let testdata = render_helpers::testdata_dir();
// ...
let zpl_path = render_helpers::labels_dir().join("bstc.zpl");
```

Do the same for `amazon.zpl`.

- [ ] **Step 2: Verify `diff_scanner::load_diff_report` still works**

The `load_diff_report` function in `src/skill/diff_scanner.rs:112` reads `testdata/diffs/diff_report.txt`. This file is generated by `e2e_diff_report.rs` and stays in `testdata/diffs/`. No change needed to the diff scanner itself -- but the diff report file will now contain paths to the subdirectories. Verify the test still passes.

- [ ] **Step 3: Run skill tests**

```bash
cargo test --test unit_skill -- --test-threads=4 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/unit_skill.rs
git commit -m "refactor: update skill tests to use labels_dir() for ZPL paths"
```

---

### Task 7: Update `unit_golden_png_dimensions.rs`

**Files:**
- Modify: `tests/unit_golden_png_dimensions.rs`

This test discovers all `.png` files in `testdata/` and checks their dimensions. After the move, golden PNGs are in subdirectories.

- [ ] **Step 1: Read the file to understand its discovery logic**

```bash
cat tests/unit_golden_png_dimensions.rs
```

- [ ] **Step 2: Update to scan subdirectories**

If it uses `read_dir` on `testdata/`, update it to also scan `testdata/labels/` and `testdata/unit/`. Follow the same pattern as Task 4's diff report update.

- [ ] **Step 3: Run the test**

```bash
cargo test --test unit_golden_png_dimensions -- --test-threads=4 2>&1 | tail -10
```

Expected: all PNGs found and validated.

- [ ] **Step 4: Commit**

```bash
git add tests/unit_golden_png_dimensions.rs
git commit -m "refactor: update PNG dimension test to scan subdirectories"
```

---

### Task 8: Update snippet extractor hardcoded paths

**Files:**
- Modify: `src/skill/snippet_extractor.rs:166,218`

The snippet extractor writes extracted ZPL to `testdata/snippets/` with hardcoded paths. These don't reference `labels/` or `unit/` directly -- but verify they still work since the snippets directory is unchanged.

- [ ] **Step 1: Verify snippet extractor paths are unaffected**

The snippet extractor writes to `testdata/snippets/{name}_{index}.zpl`. The `snippets/` directory is not being moved. No change needed, but run the test to confirm.

```bash
cargo test --test unit_skill -- snippet --test-threads=4 2>&1 | tail -10
```

- [ ] **Step 2: If tests pass, no commit needed for this task**

---

### Task 9: Regenerate diff report and run full test suite

**Files:**
- Modify: `testdata/diffs/diff_report.txt` (auto-generated)

- [ ] **Step 1: Regenerate the diff report**

The diff report in `testdata/diffs/diff_report.txt` will now contain updated paths. Run:

```bash
cargo test --test e2e_diff_report -- --test-threads=1 --nocapture 2>&1 | tail -30
```

- [ ] **Step 2: Run the full test suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: all tests pass. No new failures introduced by the reorganization.

- [ ] **Step 3: Commit the regenerated diff report**

```bash
git add testdata/diffs/diff_report.txt
git commit -m "chore: regenerate diff report after testdata reorganization"
```

---

### Task 10: Update documentation

**Files:**
- Modify: `docs/DIFF_THRESHOLDS.md` (if it references file paths)
- Modify: `README.md` (if it references testdata structure)

- [ ] **Step 1: Check for path references in docs**

```bash
grep -rn "testdata/" docs/ README.md 2>/dev/null
```

- [ ] **Step 2: Update any references to reflect the new structure**

If `docs/DIFF_THRESHOLDS.md` lists ZPL files, group them under `labels/` and `unit/` headings.

- [ ] **Step 3: Commit**

```bash
git add docs/ README.md
git commit -m "docs: update testdata path references for labels/ and unit/ split"
```

---

### Task 11: Update `e2e/testdata/sample.zpl` if needed

**Files:**
- None (verification only)

- [ ] **Step 1: Verify e2e testdata is unaffected**

The `e2e/testdata/sample.zpl` file is in a completely separate directory and is not part of the `testdata/` reorganization. Confirm the e2e tests still pass:

```bash
cargo test --test e2e 2>&1 | tail -10
```

No changes expected.
