use image::RgbaImage;
use super::bit_matrix::BitMatrix;

pub const ESCAPE_FNC_1: char = '\u{00F1}';

// Code 128 character set patterns
const CODE_A_START: u8 = 103;
const CODE_B_START: u8 = 104;
const CODE_C_START: u8 = 105;
const STOP: u8 = 106;

// Code 128 bar patterns (each code has 6 alternating bar/space widths)
static PATTERNS: [[u8; 6]; 108] = [
    [2,1,2,2,2,2],[2,2,2,1,2,2],[2,2,2,2,2,1],[1,2,1,2,2,3],[1,2,1,3,2,2],
    [1,3,1,2,2,2],[1,2,2,2,1,3],[1,2,2,3,1,2],[1,3,2,2,1,2],[2,2,1,2,1,3],
    [2,2,1,3,1,2],[2,3,1,2,1,2],[1,1,2,2,3,2],[1,2,2,1,3,2],[1,2,2,2,3,1],
    [1,1,3,2,2,2],[1,2,3,1,2,2],[1,2,3,2,2,1],[2,2,3,2,1,1],[2,2,1,1,3,2],
    [2,2,1,2,3,1],[2,1,3,2,1,2],[2,2,3,1,1,2],[3,1,2,1,3,1],[3,1,1,2,2,2],
    [3,2,1,1,2,2],[3,2,1,2,2,1],[3,1,2,2,1,2],[3,2,2,1,1,2],[3,2,2,2,1,1],
    [2,1,2,1,2,3],[2,1,2,3,2,1],[2,3,2,1,2,1],[1,1,1,3,2,3],[1,3,1,1,2,3],
    [1,3,1,3,2,1],[1,1,2,3,1,3],[1,3,2,1,1,3],[1,3,2,3,1,1],[2,1,1,3,1,3],
    [2,3,1,1,1,3],[2,3,1,3,1,1],[1,1,2,1,3,3],[1,1,2,3,3,1],[1,3,2,1,3,1],
    [1,1,3,1,2,3],[1,1,3,3,2,1],[1,3,3,1,2,1],[3,1,3,1,2,1],[2,1,1,3,3,1],
    [2,3,1,1,3,1],[2,1,3,1,1,3],[2,1,3,3,1,1],[2,1,3,1,3,1],[3,1,1,1,2,3],
    [3,1,1,3,2,1],[3,3,1,1,2,1],[3,1,2,1,1,3],[3,1,2,3,1,1],[3,3,2,1,1,1],
    [3,1,4,1,1,1],[2,2,1,4,1,1],[4,3,1,1,1,1],[1,1,1,2,2,4],[1,1,1,4,2,2],
    [1,2,1,1,2,4],[1,2,1,4,2,1],[1,4,1,1,2,2],[1,4,1,2,2,1],[1,1,2,2,1,4],
    [1,1,2,4,1,2],[1,2,2,1,1,4],[1,2,2,4,1,1],[1,4,2,1,1,2],[1,4,2,2,1,1],
    [2,4,1,2,1,1],[2,2,1,1,1,4],[4,1,3,1,1,1],[2,4,1,1,1,2],[1,3,4,1,1,1],
    [1,1,1,2,4,2],[1,2,1,1,4,2],[1,2,1,2,4,1],[1,1,4,2,1,2],[1,2,4,1,1,2],
    [1,2,4,2,1,1],[4,1,1,2,1,2],[4,2,1,1,1,2],[4,2,1,2,1,1],[2,1,2,1,4,1],
    [2,1,4,1,2,1],[4,1,2,1,2,1],[1,1,1,1,4,3],[1,1,1,3,4,1],[1,3,1,1,4,1],
    [1,1,4,1,1,3],[1,1,4,3,1,1],[4,1,1,1,1,3],[4,1,1,3,1,1],[1,1,3,1,4,1],
    [1,1,4,1,3,1],[3,1,1,1,4,1],[4,1,1,1,3,1],[2,1,1,4,1,2],[2,1,1,2,1,4],
    [2,1,1,2,3,2],[2,3,3,1,1,1],[2,1,1,1,3,2],
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

    // Auto-detect start: if content begins with 4+ digits, start with Code C
    let leading_digits = count_digits(&chars, 0);
    let mut current_set = if leading_digits >= 4 { 'C' } else { 'B' };

    let start_code = if current_set == 'C' { CODE_C_START } else { CODE_B_START };
    codes.push(start_code);
    let mut checksum = start_code as u32;
    let mut weight = 1u32;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Auto-optimize: if in A/B and 4+ digits ahead, switch to C
        if current_set != 'C' {
            let digit_run = count_digits(&chars, i);
            if digit_run >= 4 {
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

        let code = if ch == ESCAPE_FNC_1 {
            FNC1
        } else {
            let b = ch as u8;
            if b >= 32 && b <= 127 { b - 32 } else { 0 }
        };
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
    chars[from..].iter().take_while(|c| c.is_ascii_digit()).count()
}

const CODE_A_SWITCH: u8 = 101;
const CODE_B_SWITCH: u8 = 100;
const CODE_C_SWITCH: u8 = 99;
const FNC1: u8 = 102;

pub fn encode_no_mode(content: &str, height: i32, bar_width: i32) -> Result<(RgbaImage, String), String> {
    // In no-mode, subset codes like >: >; >9 >0 etc. select subsets
    // Auto-optimize: switch to Code C for runs of 4+ digits
    let mut codes: Vec<u8> = Vec::new();
    let mut text = String::new();

    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    // Determine requested start set from prefix
    let requested_set = if !chars.is_empty() && chars[0] == '>' && chars.len() > 1 {
        match chars[1] {
            ':' => { i = 2; Some('C') }
            ';' => { i = 2; Some('B') }
            '9' | '0' => { i = 2; Some('A') }
            _ => None,
        }
    } else {
        None
    };

    // Auto-detect optimal start set: if remaining data starts with 4+ digits
    // (looking past any FNC1 prefix), use Code C
    let mut lookahead = i;
    while lookahead + 1 < chars.len() && chars[lookahead] == '>' && chars[lookahead + 1] == '8' {
        lookahead += 2; // skip >8 (FNC1) when looking for digits
    }
    let digit_run = count_digits(&chars, lookahead);
    let mut current_set = match requested_set {
        Some(s) => {
            if s != 'C' && digit_run >= 4 {
                'C' // Override to Code C for digit-heavy data
            } else {
                s
            }
        }
        None => {
            if digit_run >= 4 { 'C' } else { 'B' }
        }
    };

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
                // Switch to Code C
                ':' | '2' | '4' => {
                    if current_set != 'C' {
                        codes.push(CODE_C_SWITCH);
                        checksum += CODE_C_SWITCH as u32 * weight;
                        weight += 1;
                        current_set = 'C';
                    }
                    true
                }
                // Switch to Code B
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
                '9' | '0' | '3' | '5' => {
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
                // >7 = shift to lower Code A (handle as regular chars for now)
                _ => false,
            };
            if handled {
                i += 2;
                continue;
            }
        }

        // Auto-optimize: if in Code A/B and there are 4+ consecutive digits, switch to C
        if current_set != 'C' {
            let digit_run = count_digits(&chars, i);
            if digit_run >= 4 {
                codes.push(CODE_C_SWITCH);
                checksum += CODE_C_SWITCH as u32 * weight;
                weight += 1;
                current_set = 'C';
            }
        }

        let code = match current_set {
            'C' => {
                if i + 1 < chars.len() && chars[i].is_ascii_digit() && chars[i + 1].is_ascii_digit() {
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
                    if b >= 32 && b <= 127 { b - 32 } else { 0 }
                }
            }
            'A' => {
                let ch = chars[i];
                text.push(ch);
                let b = ch as u8;
                if b >= 32 && b <= 95 { b - 32 }
                else if b < 32 { b + 64 }
                else { 0 }
            }
            _ => { // B
                let ch = chars[i];
                text.push(ch);
                let b = ch as u8;
                if b >= 32 && b <= 127 { b - 32 } else { 0 }
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
    Ok((bm.to_1d_image(bar_width.max(1) as usize, height.max(1) as usize), text))
}
