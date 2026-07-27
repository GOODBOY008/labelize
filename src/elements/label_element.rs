use super::barcode_128::{Barcode128, Barcode128WithData};
use super::barcode_2of5::{Barcode2of5, Barcode2of5WithData};
use super::barcode_39::{Barcode39, Barcode39WithData};
use super::barcode_aztec::{BarcodeAztec, BarcodeAztecWithData};
use super::barcode_datamatrix::{BarcodeDatamatrix, BarcodeDatamatrixWithData};
use super::barcode_ean13::{BarcodeEan13, BarcodeEan13WithData};
use super::barcode_pdf417::{BarcodePdf417, BarcodePdf417WithData};
use super::barcode_qr::{BarcodeQr, BarcodeQrWithData};
use super::field_block::FieldBlock;
use super::graphic_box::GraphicBox;
use super::graphic_circle::GraphicCircle;
use super::graphic_diagonal_line::GraphicDiagonalLine;
use super::graphic_ellipse::GraphicEllipse;
use super::graphic_field::GraphicField;
use super::graphic_symbol::GraphicSymbol;
use super::maxicode::{Maxicode, MaxicodeWithData};
use super::stored_format::{RecalledField, RecalledFieldData, RecalledFormat, StoredField};
use super::text_field::TextField;

/// All drawable label elements and intermediate parser constructs
#[derive(Clone, Debug)]
pub enum LabelElement {
    // Drawable elements
    Text(TextField),
    GraphicBox(GraphicBox),
    GraphicCircle(GraphicCircle),
    GraphicEllipse(GraphicEllipse),
    DiagonalLine(GraphicDiagonalLine),
    GraphicField(GraphicField),
    Barcode128(Barcode128WithData),
    BarcodeEan13(BarcodeEan13WithData),
    Barcode2of5(Barcode2of5WithData),
    Barcode39(Barcode39WithData),
    BarcodePdf417(BarcodePdf417WithData),
    BarcodeAztec(BarcodeAztecWithData),
    BarcodeDatamatrix(BarcodeDatamatrixWithData),
    BarcodeQr(BarcodeQrWithData),
    Maxicode(MaxicodeWithData),

    // Config elements (set on printer state, not drawn directly)
    Barcode128Config(Barcode128),
    BarcodeEan13Config(BarcodeEan13),
    Barcode2of5Config(Barcode2of5),
    Barcode39Config(Barcode39),
    BarcodePdf417Config(BarcodePdf417),
    BarcodeAztecConfig(BarcodeAztec),
    BarcodeDatamatrixConfig(BarcodeDatamatrix),
    BarcodeQrConfig(BarcodeQr),
    MaxicodeConfig(Maxicode),
    GraphicSymbolConfig(GraphicSymbol),
    FieldBlockConfig(FieldBlock),

    // Template elements
    StoredField(StoredField),
    RecalledFieldData(RecalledFieldData),
    RecalledField(RecalledField),
    RecalledFormat(RecalledFormat),
}

impl LabelElement {
    pub fn is_reverse_print(&self) -> bool {
        match self {
            LabelElement::Text(t) => t.reverse_print.value,
            LabelElement::GraphicBox(g) => g.reverse_print.value,
            LabelElement::GraphicCircle(g) => g.reverse_print.value,
            LabelElement::GraphicEllipse(g) => g.reverse_print.value,
            LabelElement::DiagonalLine(g) => g.reverse_print.value,
            LabelElement::GraphicField(g) => g.reverse_print.value,
            LabelElement::Barcode128(b) => b.reverse_print.value,
            LabelElement::BarcodeEan13(b) => b.reverse_print.value,
            LabelElement::Barcode2of5(b) => b.reverse_print.value,
            LabelElement::Barcode39(b) => b.reverse_print.value,
            LabelElement::BarcodePdf417(b) => b.reverse_print.value,
            LabelElement::BarcodeAztec(b) => b.reverse_print.value,
            LabelElement::BarcodeDatamatrix(b) => b.reverse_print.value,
            LabelElement::BarcodeQr(b) => b.reverse_print.value,
            LabelElement::Maxicode(m) => m.reverse_print.value,
            _ => false,
        }
    }
}
