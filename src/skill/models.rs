/// Data models for the ZPL diff auto-fix skill.

/// Status category for a diff report entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffStatus {
    Perfect,
    Good,
    Minor,
    Moderate,
    High,
    Skip,
    Error,
}

impl DiffStatus {
    /// Classify a diff percentage into a status category.
    pub fn from_percent(percent: f64) -> Self {
        if percent < 0.0 {
            DiffStatus::Skip
        } else if percent == 0.0 {
            DiffStatus::Perfect
        } else if percent < 1.0 {
            DiffStatus::Good
        } else if percent < 5.0 {
            DiffStatus::Minor
        } else if percent < 15.0 {
            DiffStatus::Moderate
        } else {
            DiffStatus::High
        }
    }

    /// Parse from the status string in diff_report.txt.
    pub fn from_report_str(s: &str) -> Self {
        let s = s.trim();
        if s.starts_with("PERFECT") {
            DiffStatus::Perfect
        } else if s.starts_with("GOOD") {
            DiffStatus::Good
        } else if s.starts_with("MINOR") {
            DiffStatus::Minor
        } else if s.starts_with("MODERATE") {
            DiffStatus::Moderate
        } else if s.starts_with("HIGH") {
            DiffStatus::High
        } else if s.starts_with("SKIP") {
            DiffStatus::Skip
        } else {
            DiffStatus::Error
        }
    }
}

/// A single entry from the diff report.
#[derive(Debug, Clone)]
pub struct DiffReportEntry {
    pub label_name: String,
    pub extension: String,
    pub diff_percent: f64,
    pub actual_dims: (u32, u32),
    pub expected_dims: (u32, u32),
    pub status: DiffStatus,
    /// Tolerance from DIFF_THRESHOLDS.md (if available).
    pub tolerance: Option<f64>,
}

/// The full diff report.
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub entries: Vec<DiffReportEntry>,
    pub total_labels: usize,
    pub perfect_count: usize,
    pub good_count: usize,
    pub minor_count: usize,
    pub moderate_count: usize,
    pub high_count: usize,
    pub skip_count: usize,
    pub error_count: usize,
}

/// Bounding box of a rendered element on the canvas.
#[derive(Debug, Clone)]
pub struct ElementBBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub element_index: usize,
    pub element_type: ElementType,
    pub zpl_command: String,
}

/// Type of a rendered element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementType {
    Text,
    GraphicBox,
    GraphicCircle,
    DiagonalLine,
    GraphicField,
    GraphicSymbol,
    Barcode128,
    BarcodeEan13,
    Barcode2of5,
    Barcode39,
    BarcodePdf417,
    BarcodeAztec,
    BarcodeDatamatrix,
    BarcodeQr,
    Maxicode,
}

/// How an element's rendering diff is classified.
///
/// Determined by comparing an isolated snippet render against the Labelary
/// reference for the same snippet:
/// - Isolated renders match well but full-label bbox diff is high → PositionDiff
/// - Isolated renders differ significantly → ContentDiff
/// - Both types of diff are significant → Mixed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffClassification {
    /// Element renders differently from reference (wrong font, barcode, graphic)
    /// but is placed at the correct coordinates.
    ContentDiff,
    /// Element renders correctly in isolation but is placed at wrong coordinates
    /// (shifted horizontally or vertically from where Labelary places it).
    PositionDiff,
    /// Both content and position differ.
    Mixed,
}

/// Method used to detect a position offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffsetDetectionMethod {
    /// Template matching via normalized cross-correlation.
    CrossCorrelation,
    /// Centroid of diff pixels compared to expected element center.
    CentroidShift,
    /// Element appears at both original and shifted positions in the diff image.
    ShadowDetection,
}

/// Detected position offset for a PositionDiff element.
#[derive(Debug, Clone)]
pub struct PositionOffsetInfo {
    /// Horizontal offset in pixels (positive = element shifted right vs. reference).
    pub dx: i32,
    /// Vertical offset in pixels (positive = element shifted down vs. reference).
    pub dy: i32,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f64,
    /// Detection method used.
    pub method: OffsetDetectionMethod,
}

/// Analysis result for a single element's contribution to the diff.
#[derive(Debug, Clone)]
pub struct ElementDiffContribution {
    pub bbox: ElementBBox,
    pub diff_pixels_in_bbox: u64,
    pub total_pixels_in_bbox: u64,
    pub local_diff_percent: f64,
    pub contribution_to_total: f64,
    /// Classification of this element's diff (None if diff_pixels_in_bbox == 0).
    pub classification: Option<DiffClassification>,
    /// Detected position offset (Some only when classification == PositionDiff or Mixed).
    pub position_offset: Option<PositionOffsetInfo>,
}

/// A standalone ZPL snippet for isolated testing.
#[derive(Debug, Clone)]
pub struct ZplSnippet {
    pub label_name: String,
    pub element_index: usize,
    pub zpl_content: String,
    pub file_path: String,
    pub original_diff_percent: f64,
}

/// A span of ZPL commands that together produce one rendered element.
#[derive(Debug, Clone)]
pub struct ZplCommandSpan {
    pub start_offset: usize,
    pub end_offset: usize,
    pub commands: Vec<ZplCommand>,
    pub element_index: usize,
}

/// A single ZPL command within a span.
#[derive(Debug, Clone)]
pub struct ZplCommand {
    pub prefix: String,
    pub params: String,
    pub offset: usize,
}

/// Fix category for auto-fix hypothesis generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixCategory {
    FontMetrics,
    BarcodeEncoding,
    PositionOffset,
    GraphicRendering,
    CommandParsing,
}

impl FixCategory {
    pub fn from_element_type(et: &ElementType) -> Self {
        match et {
            ElementType::Text => FixCategory::FontMetrics,
            ElementType::Barcode128
            | ElementType::BarcodeEan13
            | ElementType::Barcode2of5
            | ElementType::Barcode39
            | ElementType::BarcodePdf417
            | ElementType::BarcodeAztec
            | ElementType::BarcodeDatamatrix
            | ElementType::BarcodeQr
            | ElementType::Maxicode => FixCategory::BarcodeEncoding,
            ElementType::GraphicBox
            | ElementType::GraphicCircle
            | ElementType::DiagonalLine
            | ElementType::GraphicField
            | ElementType::GraphicSymbol => FixCategory::GraphicRendering,
        }
    }

    /// Select a fix category based on diff classification and element type.
    /// - ContentDiff → element-type-based mapping (same as from_element_type)
    /// - PositionDiff → PositionOffset (coordinate calculation fixes)
    /// - Mixed → PositionOffset (try position first)
    pub fn from_classification(et: &ElementType, classification: &DiffClassification) -> Self {
        match classification {
            DiffClassification::ContentDiff => Self::from_element_type(et),
            DiffClassification::PositionDiff | DiffClassification::Mixed => {
                FixCategory::PositionOffset
            }
        }
    }
}
