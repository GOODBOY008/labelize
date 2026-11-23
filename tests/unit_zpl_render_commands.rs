use labelize::elements::{field_orientation::FieldOrientation, label_element::LabelElement};
use labelize::{DrawerOptions, LabelInfo, Renderer, ZplParseOptions, ZplParser};

fn parse(data: impl AsRef<[u8]>) -> Vec<LabelInfo> {
    ZplParser::new().parse(data.as_ref()).unwrap()
}
fn options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 25.375,
        label_height_mm: 25.375,
        ..Default::default()
    }
}
fn render(label: &LabelInfo, options: DrawerOptions) -> image::GrayImage {
    let mut png = Vec::new();
    Renderer::new()
        .draw_label_as_png(label, &mut png, options)
        .unwrap();
    image::load_from_memory(&png).unwrap().to_luma8()
}
fn pixels(data: impl AsRef<[u8]>) -> image::GrayImage {
    render(&parse(data)[0], options())
}

#[test]
fn b0_uses_aztec_encoder_with_the_same_parameters_as_bo() {
    let source = "^XA^FO32,32^B0R,3,N,23^FDABC123^FS^XZ";
    let labels = parse(source);
    let LabelElement::BarcodeAztec(barcode) = &labels[0].elements[0] else {
        panic!("^B0 must emit an Aztec symbol, not text")
    };
    assert_eq!(barcode.barcode.orientation, FieldOrientation::Rotated90);
    assert_eq!(barcode.barcode.magnification, 3);
    assert_eq!(barcode.barcode.size, 23);
    assert_eq!(pixels(source), pixels(source.replace("^B0", "^BO")));
}

#[test]
fn im_draws_at_each_origin_without_renaming_or_deleting_the_image() {
    let source = "~DGR:DOT.GRF,2,1,FF81^XA^FO16,24^IMR:DOT.GRF^FS^FO48,56^IMR:DOT.GRF^FS^XZ";
    let labels = parse(source);
    assert_eq!(labels[0].elements.len(), 2);
    assert_eq!(
        pixels(source),
        pixels(source.replace("^IMR:DOT.GRF", "^XGR:DOT.GRF,1,1"))
    );
    for (element, (x, y)) in labels[0].elements.iter().zip([(16, 24), (48, 56)]) {
        let LabelElement::GraphicField(g) = element else {
            panic!("expected image")
        };
        assert_eq!((g.position.x, g.position.y), (x, y));
        assert_eq!(g.data, [255, 129]);
    }
}

#[test]
fn is_saves_the_completed_bitmap_and_n_suppresses_only_the_definition() {
    let source = "^XA^PW200^LL180^FO8,8^GB20,20,20^FS^ISR:TEMP.GRF,N^FO80,80^GB20,20,20^FS^XZ^XA^ILR:TEMP.GRF^XZ";
    let labels = ZplParser::new()
        .parse_with_options(
            source.as_bytes(),
            ZplParseOptions {
                max_labels: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(labels.len(), 1);
    let expected = "^XA^PW200^LL180^FO8,8^GB20,20,20^FS^FO80,80^GB20,20,20^FS^XZ";
    assert_eq!(render(&labels[0], options()), pixels(expected));
}

#[test]
fn is_default_prints_and_recalled_image_matches_for_different_caller_canvases() {
    let labels = parse("^XA^PW200^FO8,8^A0N,24,24^FDSAVED^FS^ISR:TEXT.GRF^XZ^XA^ILR:TEXT.GRF^XZ");
    assert_eq!(labels.len(), 2);
    for (width, height) in [(25.375, 25.375), (60.0, 40.0)] {
        let opts = DrawerOptions {
            label_width_mm: width,
            label_height_mm: height,
            ..options()
        };
        assert_eq!(render(&labels[0], opts.clone()), render(&labels[1], opts));
    }
}

#[test]
fn saved_images_freeze_transform_and_offset_state() {
    for (state, reset) in [
        ("^PMY", "^PMN"),
        ("^POI", "^PON"),
        ("^LT20", "^LT0"),
        ("^LS5", "^LS0"),
    ] {
        let labels = parse(format!(
            "^XA^PW200{state}^FO8,8^GB20,20,20^FS^ISR:T.GRF^XZ^XA{reset}^ILR:T.GRF^XZ"
        ));
        assert_eq!(
            render(&labels[0], options()),
            render(&labels[1], options()),
            "{state}"
        );
    }
}

#[test]
fn saved_images_obey_declared_dimensions_when_requested() {
    let labels = ZplParser::new()
        .parse_with_options(
            b"^XA^PW200^LL180^POI^FO8,8^GB20,20,20^FS^ISR:T.GRF^XZ^XA^PON^ILR:T.GRF^XZ",
            ZplParseOptions {
                use_declared_dimensions: true,
                ..Default::default()
            },
        )
        .unwrap();
    let opts = DrawerOptions {
        label_width_mm: 25.0,
        label_height_mm: 22.5,
        ..options()
    };
    assert_eq!(render(&labels[0], opts.clone()), render(&labels[1], opts));
}

#[test]
fn saved_images_support_placement_magnification_and_white_transparency() {
    let actual=pixels("^XA^PW200^FO0,0^GB8,8,8^FS^ISR:T.GRF,N^XZ^XA^FO24,24^GB80,80,80^FS^FO16,16^XGR:T.GRF,2,3^FS^XZ");
    // The field starts at page x=17. Its saved one-dot margin is magnified by two.
    let expected = pixels("^XA^PW200^FO24,24^GB80,80,80^FS^FO18,16^GB16,24,16^FS^XZ");
    assert!(actual == expected);
}

#[test]
fn saved_image_replacement_does_not_change_already_parsed_labels() {
    let labels=parse("^XA^PW200^FO8,8^GB8,8,8^FS^ISR:T.GRF,N^XZ^XA^ILR:T.GRF^XZ^XA^FO80,80^GB8,8,8^FS^ISR:T.GRF,N^XZ^XA^ILR:T.GRF^XZ");
    assert_eq!(labels.len(), 2);
    assert_eq!(
        render(&labels[0], options()),
        pixels("^XA^PW200^FO8,8^GB8,8,8^FS^XZ")
    );
    assert_eq!(
        render(&labels[1], options()),
        pixels("^XA^PW200^FO80,80^GB8,8,8^FS^XZ")
    );
}

#[test]
fn image_resources_survive_parse_calls_and_can_be_deleted_or_overwritten() {
    for erase in ["^IDR:T.GRF", "~EG", "~EGR:T.GRF"] {
        let mut parser = ZplParser::new();
        assert!(parser
            .parse(b"^XA^FO8,8^GB8,8,8^FS^ISR:T.GRF,N^XZ")
            .unwrap()
            .is_empty());
        assert_eq!(parser.parse(b"^XA^IMR:T.GRF^XZ").unwrap().len(), 1);
        assert!(parser
            .parse(format!("{erase}^XA^ILR:T.GRF^XZ").as_bytes())
            .unwrap()
            .is_empty());
    }
    let labels = parse("^XA^FO8,8^GB8,8,8^FS^ISR:T.GRF,N^XZ~DGR:T.GRF,1,1,FF^XA^XGR:T.GRF^XZ");
    assert!(matches!(&labels[0].elements[0], LabelElement::GraphicField(g) if g.data == [255]));
}

#[test]
fn saved_image_nesting_is_bounded_and_format_saves_run_at_recall() {
    let mut source = String::from("^XA^FO8,8^GB8,8,8^FS^ISR:T.GRF,N^XZ");
    for _ in 0..17 {
        source.push_str("^XA^ILR:T.GRF^ISR:T.GRF,N^XZ");
    }
    assert!(ZplParser::new()
        .parse(source.as_bytes())
        .unwrap_err()
        .contains("nesting"));
    let mut parser = ZplParser::new();
    assert!(parser
        .parse(b"^XA^DFR:T.ZPL^FO8,8^FDTEST^FS^ISR:T.GRF,N^XZ")
        .unwrap()
        .is_empty());
    assert!(parser.parse(b"^XA^ILR:T.GRF^XZ").unwrap().is_empty());
    assert!(parser.parse(b"^XA^XFR:T.ZPL^XZ").unwrap().is_empty());
    let recalled = parser.parse(b"^XA^ILR:T.GRF^XZ").unwrap();
    assert!(matches!(
        &recalled[0].elements[0],
        LabelElement::RasterLabel(_)
    ));
}

#[test]
fn named_font_orientation_is_the_first_argument_and_filename_is_the_fourth() {
    for orientation in ["N", "R", "I", "B"] {
        let actual =
            format!("^XA^CF0,24,20^FO80,80^A@{orientation},30,25,E:MISSING.TTF^FDHello^FS^XZ");
        let expected = format!("^XA^CF0,24,20^FO80,80^A0{orientation},30,25^FDHello^FS^XZ");
        assert_eq!(pixels(actual), pixels(expected), "{orientation}");
    }
}

#[test]
fn named_font_omissions_inherit_the_filename_orientation_and_default_sizes() {
    let labels = parse("^XA^CF0,24,20^FWR^A@N,30,25,B^FO8,8^FDONE^FS^A@,,,^FO80,80^FDTWO^FS^XZ");
    let LabelElement::Text(text) = &labels[0].elements[1] else {
        panic!("text")
    };
    assert_eq!(text.font.name, "B");
    assert_eq!(text.font.orientation, FieldOrientation::Rotated90);
}

#[test]
fn ascii_graphic_with_whitespace_renders_like_compact_hex() {
    let compact = "^XA^FO8,8^GFA,4,4,2,8001FFF0^FS^XZ";
    let spaced = "^XA^FO8,8^GFA,4,4,2, 80 01    F F\tF\r\n0 ^FS^XZ";
    assert_eq!(pixels(spaced), pixels(compact));
}

#[test]
fn binary_graphics_preserve_every_byte_and_render_like_hex() {
    let payload: Vec<u8> = (0..=255).collect();
    let mut binary = b"^XA^FO16,24^GFB,256,256,16,".to_vec();
    binary.extend_from_slice(&payload);
    binary.extend_from_slice(b"^FS^FO8,180^FDTAIL^FS^XZ");
    let hex: String = payload.iter().map(|b| format!("{b:02X}")).collect();
    let ascii = format!("^XA^FO16,24^GFA,256,256,16,{hex}^FS^FO8,180^FDTAIL^FS^XZ");
    let labels = parse(&binary);
    let LabelElement::GraphicField(g) = &labels[0].elements[0] else {
        panic!("graphic")
    };
    assert_eq!(g.data, payload);
    assert_eq!(labels[0].elements.len(), 2);
    assert_eq!(pixels(&binary), pixels(ascii));
    let formatted = labelize::parsers::zpl::format::format_zpl(&binary);
    assert_eq!(pixels(&binary), pixels(formatted));
}

#[test]
fn binary_payloads_preserve_prefixes_and_trailing_transport_bytes_after_remapping() {
    let binary = b"^CC!!XA!FO16,24!GFB,8,8,1,!^~\0\x80\t\r\n!FS!FO8,180!FDTAIL!FS!XZ";
    let labels = parse(binary);
    let LabelElement::GraphicField(g) = &labels[0].elements[0] else {
        panic!("graphic")
    };
    assert_eq!(g.data, b"!^~\0\x80\t\r\n");
    assert_eq!(
        pixels(binary),
        pixels(labelize::parsers::zpl::format::format_zpl(binary))
    );
}

#[test]
fn invalid_binary_counts_truncation_and_compressed_binary_return_errors() {
    for input in [
        b"^XA^GFB,100,100,1,\x80".as_slice(),
        b"^XA^GFB,0,1,1,^FS^XZ",
        b"^XA^GFB,100000,1,1,X^FS^XZ",
    ] {
        assert!(ZplParser::new().parse(input).is_err(), "{input:?}");
        assert_eq!(labelize::parsers::zpl::format::format_zpl(input), input);
    }
    let error = ZplParser::new()
        .parse(b"^XA^GFC,1,1,1,X^FS^XZ")
        .unwrap_err();
    assert!(error.contains("compressed binary"));
}

#[cfg(feature = "vector")]
#[test]
fn saved_bitmap_remains_a_bitmap_in_vector_pdf_and_matches_raster_pixels() {
    let labels = parse("^XA^PW200^FO8,8^GB20,20,20^FS^FO8,40^A0N,24,24^FDSAVED^FS^ISR:T.GRF,N^XZ^XA^ILR:T.GRF^FS^FO8,100^A0N,24,24^FDLIVE^FS^XZ");
    let scene = labelize::LabelScene::new(&labels[0], options()).unwrap();
    assert_eq!(scene.stats().images, 1);
    let bytes = scene
        .to_png(labelize::ScenePngOptions {
            scale: 1.0,
            antialias: false,
        })
        .unwrap();
    let vector = image::load_from_memory(&bytes).unwrap().to_luma8();
    let raster = render(&labels[0], options());
    // The stored bitmap must remain exact. Live native glyphs below y=100
    // use the scene backend's own rasterizer and are checked through PDF text.
    assert!(
        image::imageops::crop_imm(&vector, 0, 0, vector.width(), 90).to_image()
            == image::imageops::crop_imm(&raster, 0, 0, raster.width(), 90).to_image()
    );
    assert!(scene.stats().glyphs > 0);
    let pdf = lopdf::Document::load_mem(&scene.to_pdf().unwrap()).unwrap();
    assert_eq!(pdf.get_pages().len(), 1);
    let text = pdf.extract_text(&[1]).unwrap().replace('\n', "");
    assert!(text.contains("LIVE"));
    assert!(
        !text.contains("SAVED"),
        "the saved text must remain a bitmap"
    );
}

#[test]
fn legacy_usps_capture_has_truncated_binary_data() {
    let input = include_bytes!("../testdata/invalid/usps_apo_truncated.zpl");
    let error = ZplParser::new().parse(input).unwrap_err();
    assert!(error.contains("Truncated binary ^GF"));
}

#[test]
fn saved_image_can_be_loaded_and_moved_without_mutating_the_first_placement() {
    let actual =
        pixels("^XA^FO8,8^GB20,20,20^FS^ISR:T.GRF,N^XZ^XA^ILR:T.GRF^FS^FO80,80^IMR:T.GRF^FS^XZ");
    let expected = pixels("^XA^FO8,8^GB20,20,20^FS^FO88,88^GB20,20,20^FS^XZ");
    assert!(actual == expected);
}

#[test]
fn downloaded_graphics_delete_with_the_same_device_and_case_defaults_as_recall() {
    for (download, delete) in [
        ("dot", "dot"),
        ("dot", "R:DOT.GRF"),
        ("R:Dot.gRf", "dOt"),
        ("R:dot", "R:dot"),
    ] {
        let mut parser = ZplParser::new();
        let setup = format!("^XA^DFR:dot.ZPL^FO8,8^GB1,1,1^FS^XZ~DG{download},1,1,FF~DGE:DOT.GRF,1,1,80^XA^IMdot^FS^XZ");
        assert_eq!(parser.parse(setup.as_bytes()).unwrap()[0].elements.len(), 1);
        let recall = format!("^ID{delete}^XA^IMdot^FS^IME:DOT.GRF^FS^XZ^XA^XFR:dot.ZPL^FS^XZ");
        let labels = parser.parse(recall.as_bytes()).unwrap();
        assert_eq!(labels.len(), 2);
        assert!(matches!(
            &labels[1].elements[..],
            [LabelElement::GraphicBox(_)]
        ));
        assert!(
            matches!(&labels[0].elements[..], [LabelElement::GraphicField(g)] if g.data == [0x80])
        );
    }
}

fn assert_ink_coordinates(label: &LabelInfo, expected: &[(u32, u32)]) {
    fn ink(image: image::GrayImage) -> Vec<(u32, u32)> {
        image
            .enumerate_pixels()
            .filter(|(_, _, p)| p[0] == 0)
            .map(|(x, y, _)| (x, y))
            .collect()
    }
    assert_eq!(ink(render(label, options())), expected);
    #[cfg(feature = "vector")]
    {
        let scene = labelize::LabelScene::new(label, options()).unwrap();
        let png = scene
            .to_png(labelize::ScenePngOptions {
                scale: 1.0,
                antialias: false,
            })
            .unwrap();
        assert_eq!(
            ink(image::load_from_memory(&png).unwrap().to_luma8()),
            expected
        );
    }
}

#[test]
fn loaded_bitmap_keeps_both_print_edges_through_mirrors_and_inversion() {
    // Coordinate pairs measured on a 203×203 Labelary page with a 200-dot print width.
    for (settings, expected) in [
        ("", [(1, 0), (200, 3)]),
        ("^PMY", [(201, 0), (2, 3)]),
        ("^POI", [(2, 199), (201, 202)]),
        ("^POI^PMY", [(200, 199), (1, 202)]),
    ] {
        let labels = parse(format!(
            "^XA^PW200{settings}^FO0,0^GB1,1,1^FS^FO199,3^GB1,1,1^FS^ISR:E.GRF,Y^FS^XZ^XA^PMN^PON^ILR:E.GRF^FS^FO200,4^GB1,1,1^FS^XZ"
        ));
        assert_eq!(labels.len(), 2);
        for label in &labels {
            assert_ink_coordinates(label, &expected);
        }
    }
}

#[test]
fn positioned_saved_bitmaps_keep_their_page_margins_and_clip_at_the_paper_edge() {
    let saved = "^XA^PW200^PMY^FO0,0^GB1,1,1^FS^FO199,3^GB1,1,1^FS^ISR:E.GRF,N^FS^XZ";
    for command in ["^IMR:E.GRF", "^XGR:E.GRF,1,1"] {
        let labels = parse(format!(
            "{saved}^XA^PMN^FO0,0{command}^FS^XZ^XA^FO1,4{command}^FS^XZ"
        ));
        assert_ink_coordinates(&labels[0], &[(202, 0), (3, 3)]);
        assert_ink_coordinates(&labels[1], &[(4, 7)]);
    }
}
