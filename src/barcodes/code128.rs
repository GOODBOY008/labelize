use super::bit_matrix::BitMatrix;
use image::RgbaImage;

pub const ESCAPE_FNC_1: char = '\u{00F1}';

/// Prepare field data for ^BC mode U (UCC Case Mode).
///
/// Per ZPL spec: exactly 19 data digits are used (truncate or zero-pad),
/// a GS1 Mod-10 check digit is appended as the 20th character,
/// and FNC1 is prepended. The resulting string is ready for encode_auto().
pub fn prepare_ucc_mode_data(content: &str) -> String {
    // Filter to digits only, take first 19, pad with zeros on the right to 19
    let digits: String = content
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(19)
        .collect();
    let padded = format!("{:0<19}", digits);
    let check = gs1_mod10_check(&padded);
    format!("{}{}{}", ESCAPE_FNC_1, padded, check)
}

/// GS1 Mod-10 check digit calculation.
/// Rightmost digit gets weight 3, alternating 3/1 from right to left.
fn gs1_mod10_check(digits: &str) -> char {
    let sum: u32 = digits
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let d = (c as u32) - ('0' as u32);
            if i % 2 == 0 {
                d * 3
            } else {
                d
            }
        })
        .sum();
    let check = (10 - (sum % 10)) % 10;
    char::from_digit(check, 10).unwrap_or('0')
}

// Code 128 character set patterns
const CODE_A_START: u8 = 103;
const CODE_B_START: u8 = 104;
const CODE_C_START: u8 = 105;
const STOP: u8 = 106;

// Code 128 bar patterns (each code has 6 alternating bar/space widths)
static PATTERNS: [[u8; 6]; 108] = [
    [2, 1, 2, 2, 2, 2],
    [2, 2, 2, 1, 2, 2],
    [2, 2, 2, 2, 2, 1],
    [1, 2, 1, 2, 2, 3],
    [1, 2, 1, 3, 2, 2],
    [1, 3, 1, 2, 2, 2],
    [1, 2, 2, 2, 1, 3],
    [1, 2, 2, 3, 1, 2],
    [1, 3, 2, 2, 1, 2],
    [2, 2, 1, 2, 1, 3],
    [2, 2, 1, 3, 1, 2],
    [2, 3, 1, 2, 1, 2],
    [1, 1, 2, 2, 3, 2],
    [1, 2, 2, 1, 3, 2],
    [1, 2, 2, 2, 3, 1],
    [1, 1, 3, 2, 2, 2],
    [1, 2, 3, 1, 2, 2],
    [1, 2, 3, 2, 2, 1],
    [2, 2, 3, 2, 1, 1],
    [2, 2, 1, 1, 3, 2],
    [2, 2, 1, 2, 3, 1],
    [2, 1, 3, 2, 1, 2],
    [2, 2, 3, 1, 1, 2],
    [3, 1, 2, 1, 3, 1],
    [3, 1, 1, 2, 2, 2],
    [3, 2, 1, 1, 2, 2],
    [3, 2, 1, 2, 2, 1],
    [3, 1, 2, 2, 1, 2],
    [3, 2, 2, 1, 1, 2],
    [3, 2, 2, 2, 1, 1],
    [2, 1, 2, 1, 2, 3],
    [2, 1, 2, 3, 2, 1],
    [2, 3, 2, 1, 2, 1],
    [1, 1, 1, 3, 2, 3],
    [1, 3, 1, 1, 2, 3],
    [1, 3, 1, 3, 2, 1],
    [1, 1, 2, 3, 1, 3],
    [1, 3, 2, 1, 1, 3],
    [1, 3, 2, 3, 1, 1],
    [2, 1, 1, 3, 1, 3],
    [2, 3, 1, 1, 1, 3],
    [2, 3, 1, 3, 1, 1],
    [1, 1, 2, 1, 3, 3],
    [1, 1, 2, 3, 3, 1],
    [1, 3, 2, 1, 3, 1],
    [1, 1, 3, 1, 2, 3],
    [1, 1, 3, 3, 2, 1],
    [1, 3, 3, 1, 2, 1],
    [3, 1, 3, 1, 2, 1],
    [2, 1, 1, 3, 3, 1],
    [2, 3, 1, 1, 3, 1],
    [2, 1, 3, 1, 1, 3],
    [2, 1, 3, 3, 1, 1],
    [2, 1, 3, 1, 3, 1],
    [3, 1, 1, 1, 2, 3],
    [3, 1, 1, 3, 2, 1],
    [3, 3, 1, 1, 2, 1],
    [3, 1, 2, 1, 1, 3],
    [3, 1, 2, 3, 1, 1],
    [3, 3, 2, 1, 1, 1],
    [3, 1, 4, 1, 1, 1],
    [2, 2, 1, 4, 1, 1],
    [4, 3, 1, 1, 1, 1],
    [1, 1, 1, 2, 2, 4],
    [1, 1, 1, 4, 2, 2],
    [1, 2, 1, 1, 2, 4],
    [1, 2, 1, 4, 2, 1],
    [1, 4, 1, 1, 2, 2],
    [1, 4, 1, 2, 2, 1],
    [1, 1, 2, 2, 1, 4],
    [1, 1, 2, 4, 1, 2],
    [1, 2, 2, 1, 1, 4],
    [1, 2, 2, 4, 1, 1],
    [1, 4, 2, 1, 1, 2],
    [1, 4, 2, 2, 1, 1],
    [2, 4, 1, 2, 1, 1],
    [2, 2, 1, 1, 1, 4],
    [4, 1, 3, 1, 1, 1],
    [2, 4, 1, 1, 1, 2],
    [1, 3, 4, 1, 1, 1],
    [1, 1, 1, 2, 4, 2],
    [1, 2, 1, 1, 4, 2],
    [1, 2, 1, 2, 4, 1],
    [1, 1, 4, 2, 1, 2],
    [1, 2, 4, 1, 1, 2],
    [1, 2, 4, 2, 1, 1],
    [4, 1, 1, 2, 1, 2],
    [4, 2, 1, 1, 1, 2],
    [4, 2, 1, 2, 1, 1],
    [2, 1, 2, 1, 4, 1],
    [2, 1, 4, 1, 2, 1],
    [4, 1, 2, 1, 2, 1],
    [1, 1, 1, 1, 4, 3],
    [1, 1, 1, 3, 4, 1],
    [1, 3, 1, 1, 4, 1],
    [1, 1, 4, 1, 1, 3],
    [1, 1, 4, 3, 1, 1],
    [4, 1, 1, 1, 1, 3],
    [4, 1, 1, 3, 1, 1],
    [1, 1, 3, 1, 4, 1],
    [1, 1, 4, 1, 3, 1],
    [3, 1, 1, 1, 4, 1],
    [4, 1, 1, 1, 3, 1],
    [2, 1, 1, 4, 1, 2],
    [2, 1, 1, 2, 1, 4],
    [2, 1, 1, 2, 3, 2],
    [2, 3, 3, 1, 1, 1],
    [2, 1, 1, 1, 3, 2],
];

static STOP_PATTERN: [u8; 7] = [2, 3, 3, 1, 1, 1, 2];

fn encode_pattern(codes: &[u8]) -> BitMatrix {
    let mut total_width = 0usize;
    for &code in codes.iter().take(codes.len() - 1) {
        let pattern = &PATTERNS[code as usize];
        let pw: usize = pattern.iter().map(|&w| w as usize).sum();
        total_width += pw;
    }
    // Stop pattern
    let sw: usize = STOP_PATTERN.iter().map(|&w| w as usize).sum();
    total_width += sw;

    let mut bm = BitMatrix::new(total_width, 1);
    let mut pos = 0;

    for (i, &code) in codes.iter().enumerate() {
        let pattern = if i == codes.len() - 1 {
            &STOP_PATTERN[..]
        } else {
            &PATTERNS[code as usize][..]
        };

        let mut bar = true;
        for &w in pattern {
            for _ in 0..w {
                if bar {
                    bm.set(pos, 0, true);
                }
                pos += 1;
            }
            bar = !bar;
        }
    }

    bm
}

pub fn encode_auto(content: &str, height: i32, bar_width: i32) -> Result<RgbaImage, String> {
    let mut codes: Vec<u8> = Vec::new();
    let chars: Vec<char> = content.chars().collect();

    // Auto-detect start: skip leading FNC1 chars, then check for 4+ digits
    let mut skip = 0;
    while skip < chars.len() && chars[skip] == ESCAPE_FNC_1 {
        skip += 1;
    }
    let leading_digits = count_digits(&chars, skip);
    let mut current_set = if leading_digits >= 4 { 'C' } else { 'B' };

    let start_code = if current_set == 'C' {
        CODE_C_START
    } else {
        CODE_B_START
    };
    codes.push(start_code);
    let mut checksum = start_code as u32;
    let mut weight = 1u32;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // FNC1 is valid in any code set
        if ch == ESCAPE_FNC_1 {
            codes.push(FNC1);
            checksum += FNC1 as u32 * weight;
            weight += 1;
            i += 1;
            continue;
        }

        // Auto-optimize: if in A/B and 4+ digits ahead, switch to C. Odd runs defer the
        // switch by one digit -- C encodes pairs, so an odd boundary strands the last
        // digit and costs an extra CODE_B switch.
        if current_set != 'C' {
            let digit_run = count_digits(&chars, i);
            if digit_run >= 4 && digit_run.is_multiple_of(2) {
                codes.push(CODE_C_SWITCH);
                checksum += CODE_C_SWITCH as u32 * weight;
                weight += 1;
                current_set = 'C';
            }
        }

        if current_set == 'C' {
            if i + 1 < chars.len() && ch.is_ascii_digit() && chars[i + 1].is_ascii_digit() {
                let val = (ch as u8 - b'0') * 10 + (chars[i + 1] as u8 - b'0');
                codes.push(val);
                checksum += val as u32 * weight;
                weight += 1;
                i += 2;
                continue;
            } else {
                // Not a digit pair, switch to Code B
                codes.push(CODE_B_SWITCH);
                checksum += CODE_B_SWITCH as u32 * weight;
                weight += 1;
                current_set = 'B';
            }
        }

        let b = ch as u8;
        let code = if (32..=127).contains(&b) { b - 32 } else { 0 };
        codes.push(code);
        checksum += code as u32 * weight;
        weight += 1;
        i += 1;
    }

    codes.push((checksum % 103) as u8);
    codes.push(STOP);

    let bm = encode_pattern(&codes);
    Ok(bm.to_1d_image(bar_width.max(1) as usize, height.max(1) as usize))
}

// Count consecutive digits starting at position `from` in `chars`
fn count_digits(chars: &[char], from: usize) -> usize {
    chars[from..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count()
}

const CODE_A_SWITCH: u8 = 101;
const CODE_B_SWITCH: u8 = 100;
const CODE_C_SWITCH: u8 = 99;
const FNC1: u8 = 102;

pub fn encode_no_mode(
    content: &str,
    height: i32,
    bar_width: i32,
) -> Result<(RgbaImage, String), String> {
    // In mode N, subset codes >9/>:/>; select the start subset, and >5/>6/>7 switch subsets.
    // Mode N has no automatic optimization — explicit escape codes control everything.
    let mut codes: Vec<u8> = Vec::new();
    let mut text = String::new();

    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    // Determine requested start set from prefix.
    // Per ZPL spec: >9=Start Code A (103), >:=Start Code B (104), >;=Start Code C (105).
    let requested_set = if !chars.is_empty() && chars[0] == '>' && chars.len() > 1 {
        match chars[1] {
            ':' => {
                i = 2;
                Some('B') // >: = Start Code B (104)
            }
            ';' => {
                i = 2;
                Some('C') // >; = Start Code C (105)
            }
            '9' | '0' => {
                i = 2;
                Some('A') // >9 = Start Code A (103)
            }
            _ => None,
        }
    } else {
        None
    };

    // Mode N uses the explicitly requested start set; no auto-optimization.
    let mut current_set = requested_set.unwrap_or('B');

    let start_code = match current_set {
        'A' => CODE_A_START,
        'C' => CODE_C_START,
        _ => CODE_B_START,
    };
    codes.push(start_code);
    let mut checksum = start_code as u32;
    let mut weight = 1u32;

    while i < chars.len() {
        // Handle > prefix commands (subset switching)
        if chars[i] == '>' && i + 1 < chars.len() {
            let handled = match chars[i + 1] {
                // >5 = Switch to Code C (value 99), per ZPL spec table
                ':' | '2' | '4' | '5' => {
                    if current_set != 'C' {
                        codes.push(CODE_C_SWITCH);
                        checksum += CODE_C_SWITCH as u32 * weight;
                        weight += 1;
                        current_set = 'C';
                    }
                    true
                }
                // >6 = Switch to Code B (value 100) from Code A or C, per ZPL spec table
                ';' | '1' | '6' => {
                    if current_set != 'B' {
                        codes.push(CODE_B_SWITCH);
                        checksum += CODE_B_SWITCH as u32 * weight;
                        weight += 1;
                        current_set = 'B';
                    }
                    true
                }
                // Switch to Code A
                '9' | '0' | '3' => {
                    if current_set != 'A' {
                        codes.push(CODE_A_SWITCH);
                        checksum += CODE_A_SWITCH as u32 * weight;
                        weight += 1;
                        current_set = 'A';
                    }
                    true
                }
                // FNC1
                '8' => {
                    codes.push(FNC1);
                    checksum += FNC1 as u32 * weight;
                    weight += 1;
                    true
                }
                // >7 = Switch to Code A (value 101) from Code B or C, per ZPL spec table
                '7' => {
                    if current_set != 'A' {
                        codes.push(CODE_A_SWITCH);
                        checksum += CODE_A_SWITCH as u32 * weight;
                        weight += 1;
                        current_set = 'A';
                    }
                    true
                }
                _ => false,
            };
            if handled {
                i += 2;
                continue;
            }
        }

        let code = match current_set {
            'C' => {
                if i + 1 < chars.len() && chars[i].is_ascii_digit() && chars[i + 1].is_ascii_digit()
                {
                    let val = (chars[i] as u8 - b'0') * 10 + (chars[i + 1] as u8 - b'0');
                    text.push(chars[i]);
                    text.push(chars[i + 1]);
                    i += 1;
                    val
                } else {
                    // Can't encode as digit pair in Code C, switch to Code B
                    codes.push(CODE_B_SWITCH);
                    checksum += CODE_B_SWITCH as u32 * weight;
                    weight += 1;
                    current_set = 'B';
                    let ch = chars[i];
                    text.push(ch);
                    let b = ch as u8;
                    if (32..=127).contains(&b) {
                        b - 32
                    } else {
                        0
                    }
                }
            }
            'A' => {
                // Code A in mode N uses digit pairs (00-99) like Code C.
                // Each pair XY encodes Code 128 value X*10+Y.
                // Values 0-63 map to printable ASCII 32-95; values 64-95 map to control chars 0-31.
                if i + 1 < chars.len() && chars[i].is_ascii_digit() && chars[i + 1].is_ascii_digit()
                {
                    let val = (chars[i] as u8 - b'0') * 10 + (chars[i + 1] as u8 - b'0');
                    let ch = if val < 64 {
                        char::from(val + 32) // printable: ASCII 32-95
                    } else if val < 96 {
                        char::from(val - 64) // control: ASCII 0-31 (not shown but encoded)
                    } else {
                        ' ' // special codes (96+): suppress from display
                    };
                    text.push(ch);
                    i += 1; // extra increment; loop adds one more → net advance of 2
                    val
                } else {
                    // Non-digit data in Code A: fall back to Code B
                    codes.push(CODE_B_SWITCH);
                    checksum += CODE_B_SWITCH as u32 * weight;
                    weight += 1;
                    current_set = 'B';
                    let ch = chars[i];
                    text.push(ch);
                    let b = ch as u8;
                    if (32..=127).contains(&b) {
                        b - 32
                    } else {
                        0
                    }
                }
            }
            _ => {
                // B
                let ch = chars[i];
                text.push(ch);
                let b = ch as u8;
                if (32..=127).contains(&b) {
                    b - 32
                } else {
                    0
                }
            }
        };

        codes.push(code);
        checksum += code as u32 * weight;
        weight += 1;
        i += 1;
    }

    codes.push((checksum % 103) as u8);
    codes.push(STOP);

    let bm = encode_pattern(&codes);
    Ok((
        bm.to_1d_image(bar_width.max(1) as usize, height.max(1) as usize),
        text,
    ))
}
