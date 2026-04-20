# Requirements Document

## Introduction

The ZPL Diff Auto-Fix Skill is an automated agent workflow for the Labelize project. It detects labels with high rendering diff percentages compared to Labelary reference images, isolates the problematic ZPL commands into standalone test files, and iteratively fixes the rendering logic until the diff drops below a target threshold (default 1%). The skill integrates with the existing e2e golden test infrastructure and operates as a Kiro hook/agent workflow.

## Glossary

- **DiffScanner**: The component that runs the e2e diff report and parses results to identify labels exceeding their tolerance thresholds
- **ElementAnalyzer**: The component that determines which ZPL elements contribute most to the pixel diff by correlating diff regions with element bounding boxes
- **SnippetExtractor**: The component that extracts problematic ZPL commands into standalone ZPL files for isolated rendering and testing
- **AutoFixLoop**: The component that iteratively applies rendering fixes and validates them against both the isolated snippet and the full label
- **SkillOrchestrator**: The top-level coordinator that ties all components together and integrates with the Kiro hook system
- **DiffReport**: A structured summary of pixel-diff percentages for all test labels compared against Labelary reference images
- **DiffReportEntry**: A single row from the diff report containing label name, extension, diff percentage, tolerance, and status
- **ElementBBox**: A bounding box describing the pixel region occupied by a rendered ZPL element on the canvas
- **ElementDiffContribution**: An analysis result quantifying how much a single element contributes to the total pixel diff
- **ZplSnippet**: A standalone ZPL file containing the commands for a single element, wrapped in `^XA`/`^XZ`, suitable for isolated rendering
- **ZplCommandSpan**: A contiguous sequence of ZPL commands (from `^FO`/`^FT` through `^FS`) that together produce one rendered element
- **FixHypothesis**: A proposed code change targeting a specific source file to reduce the rendering diff for a particular element type
- **FixAttempt**: The recorded result of applying a single fix hypothesis, including before/after diff and regression status
- **FixResult**: The final outcome of the auto-fix loop for a label, including success/failure, all attempts, and files modified
- **DiffClassification**: The categorization of an element's diff as ContentDiff (element renders wrong), PositionDiff (element placed wrong), or Mixed (both)
- **ContentDiff**: A diff where the element renders differently from the reference — wrong font metrics, incorrect barcode encoding, wrong graphic rendering — but is in the correct position
- **PositionDiff**: A diff where the element renders correctly but is placed at wrong coordinates — shifted horizontally or vertically from where Labelary places it
- **PositionOffsetInfo**: The detected horizontal and vertical pixel offset for a PositionDiff element, including confidence score and detection method
- **SnippetComparison**: The process of rendering an element in isolation and comparing it against the Labelary reference for the same isolated snippet to determine diff classification
- **DiffHeatmap**: A spatial analysis of diff pixel distribution across the label canvas, broken into regions aligned with element bounding boxes
- **Labelary**: The reference ZPL rendering service used as the ground truth for pixel comparison
- **Golden_Test**: An e2e test that compares Labelize-rendered PNGs pixel-by-pixel against Labelary reference images
- **Tolerance**: The maximum allowed diff percentage for a label before it is flagged as needing a fix
- **Regression**: An increase in diff percentage for a previously-passing label caused by a code change

## Requirements

### Requirement 1: Diff Report Scanning

**User Story:** As a developer, I want to scan the e2e diff report and identify labels that exceed their tolerance thresholds, so that I know which labels need rendering fixes.

#### Acceptance Criteria

1. WHEN the DiffScanner is invoked, THE DiffScanner SHALL execute the e2e diff report test and parse the output into structured DiffReportEntry records
2. WHEN parsing a diff report line, THE DiffScanner SHALL extract the label name, file extension, diff percentage, tolerance, and status category for each entry
3. THE DiffScanner SHALL classify each entry into one of the status categories: Perfect, Good, Minor, Moderate, High, Skip, or Error
4. WHEN filtering for high-diff labels, THE DiffScanner SHALL return only entries where the diff percentage exceeds the label's configured tolerance
5. WHEN scanning a single label by name, THE DiffScanner SHALL return the DiffReportEntry for that specific label
6. IF a requested label name does not exist in the diff report, THEN THE DiffScanner SHALL return a LabelNotFound error with a list of available label names
7. THE DiffScanner SHALL validate that each parsed diff percentage is in the range [0.0, 100.0] or is -1.0 for skipped/errored labels

### Requirement 2: Element Diff Analysis

**User Story:** As a developer, I want to determine which ZPL elements contribute most to a label's rendering diff, so that I can focus debugging on the most impactful elements.

#### Acceptance Criteria

1. WHEN analyzing a label, THE ElementAnalyzer SHALL parse the ZPL content and compute a bounding box for each drawable element
2. WHEN computing bounding boxes, THE ElementAnalyzer SHALL produce boxes with positive width and height for all drawable element types including Text, GraphicBox, GraphicCircle, DiagonalLine, GraphicField, and all barcode types
3. WHEN correlating diff regions, THE ElementAnalyzer SHALL load the diff image and count red diff pixels (R>200, G<50, B<50, A>200) within each element's bounding box
4. WHEN computing element contributions, THE ElementAnalyzer SHALL calculate each element's local diff percentage (diff pixels / total pixels in bbox) and contribution to total diff (diff pixels in bbox / total diff pixels in image)
5. THE ElementAnalyzer SHALL return elements sorted by contribution to total diff in descending order
6. WHEN the diff image contains zero diff pixels, THE ElementAnalyzer SHALL return an empty contributions list
7. WHEN computing bounding boxes for text elements, THE ElementAnalyzer SHALL account for font size, scale, field blocks, and rotation orientation

### Requirement 3: ZPL Snippet Extraction

**User Story:** As a developer, I want to extract problematic ZPL elements into standalone ZPL files, so that I can render and debug them in isolation.

#### Acceptance Criteria

1. WHEN extracting a snippet, THE SnippetExtractor SHALL parse the original ZPL into command spans and identify the span corresponding to the target element
2. WHEN building a standalone snippet, THE SnippetExtractor SHALL include all global state commands (`^LH`, `^PW`, `^CF`, `^BY`, `^CI`, `^FW`, `^PO`, `^LR`) that affect the target element
3. THE SnippetExtractor SHALL wrap extracted commands in `^XA` and `^XZ` delimiters to produce valid standalone ZPL
4. WHEN writing a snippet file, THE SnippetExtractor SHALL save the file to `testdata/snippets/{label_name}_{element_index}.zpl`
5. WHEN extracting a group of related elements, THE SnippetExtractor SHALL combine their command spans into a single standalone ZPL file preserving their relative order
6. THE SnippetExtractor SHALL produce ZPL command spans where each span groups commands from a position command (`^FO`/`^FT`) through `^FS`, with exactly one span per drawable element in the same order as LabelInfo elements

### Requirement 4: Auto-Fix Loop

**User Story:** As a developer, I want an automated iterative fix loop that applies rendering fixes and validates them, so that I can reduce label diff percentages without manual trial-and-error.

#### Acceptance Criteria

1. WHEN running the fix loop, THE AutoFixLoop SHALL process elements in order of highest diff contribution first
2. WHEN a fix attempt reduces the diff percentage without causing regressions, THE AutoFixLoop SHALL keep the change and update the current diff
3. WHEN a fix attempt does not reduce the diff percentage, THE AutoFixLoop SHALL rollback the change and restore the original file
4. WHEN a fix attempt causes a regression on another label beyond the configured regression tolerance, THE AutoFixLoop SHALL rollback the change
5. WHEN the current diff drops to or below the target diff percentage, THE AutoFixLoop SHALL stop iterating and report success
6. WHEN the maximum number of iterations is reached without achieving the target diff, THE AutoFixLoop SHALL stop and report failure with all attempt details
7. THE AutoFixLoop SHALL ensure that the final diff percentage is less than or equal to the initial diff percentage for the target label
8. WHEN generating a fix hypothesis, THE AutoFixLoop SHALL use the element's DiffClassification to select the fix strategy: ContentDiff maps to renderer/encoder fixes (FontMetrics for Text, BarcodeEncoding for barcodes, GraphicRendering for graphics), PositionDiff maps to coordinate fixes (PositionOffset, CommandParsing targeting parser and alignment logic), and Mixed SHALL attempt position fix first then content fix
9. WHEN a code change causes a build failure, THE AutoFixLoop SHALL immediately rollback the change and mark the attempt as failed

### Requirement 5: Regression Safety

**User Story:** As a developer, I want the auto-fix process to verify that fixes do not regress other labels, so that improvements to one label do not break others.

#### Acceptance Criteria

1. WHEN a fix is applied, THE AutoFixLoop SHALL run a regression check against all other golden test labels
2. THE regression check SHALL flag any label whose diff percentage increased by more than the configured regression tolerance (default 0.5 percentage points)
3. WHEN a regression is detected, THE AutoFixLoop SHALL fully rollback the fix and record the regressed label names in the attempt log
4. WHEN a rollback is performed, THE AutoFixLoop SHALL restore the modified file to its exact pre-change state so that the label's diff percentage returns to its value before the attempt

### Requirement 6: Skill Orchestration

**User Story:** As a developer, I want a single entry point that coordinates the full scan-analyze-extract-fix pipeline, so that I can trigger the entire workflow with one command.

#### Acceptance Criteria

1. WHEN fixing a single label, THE SkillOrchestrator SHALL execute the full pipeline: scan the diff report, analyze the label's element contributions, extract snippets, and run the auto-fix loop
2. WHEN fixing all high-diff labels, THE SkillOrchestrator SHALL identify all labels exceeding their tolerance and run the fix pipeline for each one
3. WHEN running in analysis-only mode, THE SkillOrchestrator SHALL return element diff contributions without applying any fixes
4. WHEN running in extract-only mode, THE SkillOrchestrator SHALL extract and save ZPL snippets without applying any fixes
5. IF an error occurs during the pipeline for one label, THEN THE SkillOrchestrator SHALL log the error and continue processing remaining labels

### Requirement 7: Kiro Hook Integration

**User Story:** As a developer, I want the skill to integrate as a Kiro hook, so that I can trigger the auto-fix workflow from within the IDE.

#### Acceptance Criteria

1. THE SkillOrchestrator SHALL be invocable as a Kiro userTriggered hook with an askAgent action
2. WHEN triggered, THE SkillOrchestrator SHALL accept a label name parameter or default to processing all high-diff labels
3. WHEN the workflow completes, THE SkillOrchestrator SHALL produce a human-readable report summarizing actions taken, diff improvements, and any failures

### Requirement 8: Error Handling

**User Story:** As a developer, I want clear error reporting and graceful recovery, so that the skill handles failures without leaving the codebase in a broken state.

#### Acceptance Criteria

1. IF a requested label does not exist in testdata, THEN THE SkillOrchestrator SHALL return a LabelNotFound error with a list of available labels and suggest the closest match using edit distance
2. IF the Labelary API is unavailable when fetching a snippet reference, THEN THE SkillOrchestrator SHALL fall back to cached reference images or skip snippet comparison with a warning
3. IF a code change causes a build failure, THEN THE AutoFixLoop SHALL immediately rollback the change, verify the build passes, and continue to the next hypothesis
4. IF the maximum iteration count is exhausted without reaching the target diff, THEN THE AutoFixLoop SHALL return a FixResult with success=false and all attempt details for manual review

### Requirement 9: Data Validation

**User Story:** As a developer, I want all data structures to enforce their invariants, so that invalid states are caught early and do not propagate through the pipeline.

#### Acceptance Criteria

1. THE DiffReport SHALL validate that the sum of category counts (perfect, good, minor, moderate, high) equals total_labels
2. THE DiffReportEntry SHALL validate that diff_percent is in the range [0.0, 100.0] or is -1.0 for skipped/errored entries
3. THE ZplCommandSpan SHALL validate that start_offset is less than end_offset and that the span contains at least one command
4. THE DiffHeatmap SHALL validate that density values are in the range [0.0, 1.0] and that width and height match the label canvas dimensions
5. THE ElementBBox SHALL validate that width and height are positive for all drawable element types

### Requirement 10: Diff Classification (Content vs Position)

**User Story:** As a developer, I want the analyzer to classify each element's diff as a content difference or a position difference, so that the auto-fix loop can apply the correct fix strategy for each type.

#### Acceptance Criteria

1. WHEN analyzing an element with diff pixels, THE ElementAnalyzer SHALL classify the diff as ContentDiff, PositionDiff, or Mixed by comparing the isolated snippet render against the Labelary snippet render
2. WHEN the isolated snippet render matches the Labelary snippet render within the snippet threshold (default 2%) BUT the full-label diff for that element's bbox is above the threshold, THE ElementAnalyzer SHALL classify the diff as PositionDiff
3. WHEN the isolated snippet render differs significantly from the Labelary snippet render (above the snippet threshold), THE ElementAnalyzer SHALL classify the diff as ContentDiff
4. WHEN both the isolated snippet diff and the positional diff are significant, THE ElementAnalyzer SHALL classify the diff as Mixed
5. WHEN an element is classified as PositionDiff, THE ElementAnalyzer SHALL attempt to detect the position offset vector (dx, dy) using cross-correlation, centroid shift analysis, or shadow pattern detection
6. WHEN a position offset is detected with confidence > 0.5, THE PositionOffsetInfo SHALL include the horizontal offset (dx), vertical offset (dy), confidence score in [0.0, 1.0], and the detection method used
7. WHEN generating fix hypotheses for PositionDiff elements, THE AutoFixLoop SHALL target coordinate calculation files: `src/parsers/zpl_parser.rs` for `^FO`/`^FT` parameter parsing, `src/elements/field_alignment.rs` for alignment logic, and `src/elements/drawer_options.rs` for label home offset
8. WHEN generating fix hypotheses for ContentDiff elements, THE AutoFixLoop SHALL target renderer/encoder files: `src/drawers/renderer.rs` for rendering logic, `src/barcodes/*.rs` for barcode encoding, and `src/elements/*.rs` for element-specific rendering
9. WHEN an element is classified as Mixed, THE AutoFixLoop SHALL attempt the position fix first (as it is typically easier and more impactful), then attempt the content fix if the position fix alone does not achieve the target diff
10. WHEN an element has zero diff pixels in its bounding box, THE ElementAnalyzer SHALL NOT assign a DiffClassification to that element
