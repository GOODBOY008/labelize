use image::{Rgba, RgbaImage};

// ---------------------------------------------------------------------------
// GF(64) tables — primitive polynomial x^6 + x + 1 = 0x43, generator base 1
// Derived from libzint reedsol_logs.h (logt_0x43 / alog_0x43)
// ---------------------------------------------------------------------------

/// Discrete logarithm table: logt[x] = e such that alpha^e = x.  logt[0] is unused (set to 0).
#[rustfmt::skip]
static LOGT: [u8; 64] = [
    0x00, 0x00, 0x01, 0x06, 0x02, 0x0C, 0x07, 0x1A, 0x03, 0x20, 0x0D, 0x23, 0x08, 0x30, 0x1B, 0x12,
    0x04, 0x18, 0x21, 0x10, 0x0E, 0x34, 0x24, 0x36, 0x09, 0x2D, 0x31, 0x26, 0x1C, 0x29, 0x13, 0x38,
    0x05, 0x3E, 0x19, 0x0B, 0x22, 0x1F, 0x11, 0x2F, 0x0F, 0x17, 0x35, 0x33, 0x25, 0x2C, 0x37, 0x28,
    0x0A, 0x3D, 0x2E, 0x1E, 0x32, 0x16, 0x27, 0x2B, 0x1D, 0x3C, 0x2A, 0x15, 0x14, 0x3B, 0x39, 0x3A,
];

/// Antilogarithm table (doubled for index wrap): alog[i] = alpha^(i mod 63).
#[rustfmt::skip]
static ALOG: [u8; 126] = [
    0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x03, 0x06, 0x0C, 0x18, 0x30, 0x23, 0x05, 0x0A, 0x14, 0x28,
    0x13, 0x26, 0x0F, 0x1E, 0x3C, 0x3B, 0x35, 0x29, 0x11, 0x22, 0x07, 0x0E, 0x1C, 0x38, 0x33, 0x25,
    0x09, 0x12, 0x24, 0x0B, 0x16, 0x2C, 0x1B, 0x36, 0x2F, 0x1D, 0x3A, 0x37, 0x2D, 0x19, 0x32, 0x27,
    0x0D, 0x1A, 0x34, 0x2B, 0x15, 0x2A, 0x17, 0x2E, 0x1F, 0x3E, 0x3F, 0x3D, 0x39, 0x31, 0x21,
    // Doubled for wrap-around (index 63..125)
    0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x03, 0x06, 0x0C, 0x18, 0x30, 0x23, 0x05, 0x0A, 0x14, 0x28,
    0x13, 0x26, 0x0F, 0x1E, 0x3C, 0x3B, 0x35, 0x29, 0x11, 0x22, 0x07, 0x0E, 0x1C, 0x38, 0x33, 0x25,
    0x09, 0x12, 0x24, 0x0B, 0x16, 0x2C, 0x1B, 0x36, 0x2F, 0x1D, 0x3A, 0x37, 0x2D, 0x19, 0x32, 0x27,
    0x0D, 0x1A, 0x34, 0x2B, 0x15, 0x2A, 0x17, 0x2E, 0x1F, 0x3E, 0x3F, 0x3D, 0x39, 0x31, 0x21,
];

// ---------------------------------------------------------------------------
// MaxiCode character tables (from libzint maxicode.h)
// ---------------------------------------------------------------------------

/// Per-byte bit flags indicating which code set(s) the byte belongs to:
/// A=0x01  B=0x02  E=0x04  C=0x08  D=0x10
#[rustfmt::skip]
static MAXI_CODE_SET: [u8; 256] = [
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x05, 0x04, 0x04,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F, 0x1F, 0x1F, 0x04,
    0x1F, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x03, 0x01, 0x03, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x03, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
    0x10, 0x10, 0x10, 0x10, 0x10, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x10, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x10, 0x04, 0x08, 0x10, 0x08, 0x04, 0x04, 0x10,
    0x10, 0x08, 0x08, 0x08, 0x10, 0x08, 0x04, 0x10, 0x10, 0x08, 0x08, 0x10, 0x08, 0x08, 0x08, 0x10,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
];

/// Per-byte symbol value (Set A value for chars in Set A; primary set value otherwise).
#[rustfmt::skip]
static MAXI_SYMBOL_CHAR: [u8; 256] = [
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12,  0, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 30, 28, 29, 30, 35,
    32, 53, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 37, 38, 39, 40, 41,
    52,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 42, 43, 44, 45, 46,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 32, 54, 34, 35, 36,
    48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 47, 48, 49, 50, 51, 52,
    53, 54, 55, 56, 57, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 36,
    37, 37, 38, 39, 40, 41, 42, 43, 38, 44, 37, 39, 38, 45, 46, 40,
    41, 39, 40, 41, 42, 42, 47, 43, 44, 43, 44, 45, 45, 46, 47, 46,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 32, 33, 34, 35, 36,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 32, 33, 34, 35, 36,
];

// ---------------------------------------------------------------------------
// BITNR table — maps hex-grid (row, col) to bit number in the 864-bit stream.
// Special values: -1 = always white, -2 = always dark (orientation mark),
//                 -3 = bullseye area (handled separately).
// ---------------------------------------------------------------------------
#[rustfmt::skip]
static BITNR: [[i16; 30]; 33] = [
    [ 121, 120, 127, 126, 133, 132, 139, 138, 145, 144, 151, 150, 157, 156, 163, 162, 169, 168, 175, 174, 181, 180, 187, 186, 193, 192, 199, 198,  -2,  -2],
    [ 123, 122, 129, 128, 135, 134, 141, 140, 147, 146, 153, 152, 159, 158, 165, 164, 171, 170, 177, 176, 183, 182, 189, 188, 195, 194, 201, 200, 816,  -3],
    [ 125, 124, 131, 130, 137, 136, 143, 142, 149, 148, 155, 154, 161, 160, 167, 166, 173, 172, 179, 178, 185, 184, 191, 190, 197, 196, 203, 202, 818, 817],
    [ 283, 282, 277, 276, 271, 270, 265, 264, 259, 258, 253, 252, 247, 246, 241, 240, 235, 234, 229, 228, 223, 222, 217, 216, 211, 210, 205, 204, 819,  -3],
    [ 285, 284, 279, 278, 273, 272, 267, 266, 261, 260, 255, 254, 249, 248, 243, 242, 237, 236, 231, 230, 225, 224, 219, 218, 213, 212, 207, 206, 821, 820],
    [ 287, 286, 281, 280, 275, 274, 269, 268, 263, 262, 257, 256, 251, 250, 245, 244, 239, 238, 233, 232, 227, 226, 221, 220, 215, 214, 209, 208, 822,  -3],
    [ 289, 288, 295, 294, 301, 300, 307, 306, 313, 312, 319, 318, 325, 324, 331, 330, 337, 336, 343, 342, 349, 348, 355, 354, 361, 360, 367, 366, 824, 823],
    [ 291, 290, 297, 296, 303, 302, 309, 308, 315, 314, 321, 320, 327, 326, 333, 332, 339, 338, 345, 344, 351, 350, 357, 356, 363, 362, 369, 368, 825,  -3],
    [ 293, 292, 299, 298, 305, 304, 311, 310, 317, 316, 323, 322, 329, 328, 335, 334, 341, 340, 347, 346, 353, 352, 359, 358, 365, 364, 371, 370, 827, 826],
    [ 409, 408, 403, 402, 397, 396, 391, 390,  79,  78,  -2,  -2,  13,  12,  37,  36,   2,  -1,  44,  43, 109, 108, 385, 384, 379, 378, 373, 372, 828,  -3],
    [ 411, 410, 405, 404, 399, 398, 393, 392,  81,  80,  40,  -2,  15,  14,  39,  38,   3,  -1,  -1,  45, 111, 110, 387, 386, 381, 380, 375, 374, 830, 829],
    [ 413, 412, 407, 406, 401, 400, 395, 394,  83,  82,  41,  -3,  -3,  -3,  -3,  -3,   5,   4,  47,  46, 113, 112, 389, 388, 383, 382, 377, 376, 831,  -3],
    [ 415, 414, 421, 420, 427, 426, 103, 102,  55,  54,  16,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  20,  19,  85,  84, 433, 432, 439, 438, 445, 444, 833, 832],
    [ 417, 416, 423, 422, 429, 428, 105, 104,  57,  56,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  22,  21,  87,  86, 435, 434, 441, 440, 447, 446, 834,  -3],
    [ 419, 418, 425, 424, 431, 430, 107, 106,  59,  58,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  23,  89,  88, 437, 436, 443, 442, 449, 448, 836, 835],
    [ 481, 480, 475, 474, 469, 468,  48,  -2,  30,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,   0,  53,  52, 463, 462, 457, 456, 451, 450, 837,  -3],
    [ 483, 482, 477, 476, 471, 470,  49,  -1,  -2,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -2,  -1, 465, 464, 459, 458, 453, 452, 839, 838],
    [ 485, 484, 479, 478, 473, 472,  51,  50,  31,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,   1,  -2,  42, 467, 466, 461, 460, 455, 454, 840,  -3],
    [ 487, 486, 493, 492, 499, 498,  97,  96,  61,  60,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  26,  91,  90, 505, 504, 511, 510, 517, 516, 842, 841],
    [ 489, 488, 495, 494, 501, 500,  99,  98,  63,  62,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  28,  27,  93,  92, 507, 506, 513, 512, 519, 518, 843,  -3],
    [ 491, 490, 497, 496, 503, 502, 101, 100,  65,  64,  17,  -3,  -3,  -3,  -3,  -3,  -3,  -3,  18,  29,  95,  94, 509, 508, 515, 514, 521, 520, 845, 844],
    [ 559, 558, 553, 552, 547, 546, 541, 540,  73,  72,  32,  -3,  -3,  -3,  -3,  -3,  -3,  10,  67,  66, 115, 114, 535, 534, 529, 528, 523, 522, 846,  -3],
    [ 561, 560, 555, 554, 549, 548, 543, 542,  75,  74,  -2,  -1,   7,   6,  35,  34,  11,  -2,  69,  68, 117, 116, 537, 536, 531, 530, 525, 524, 848, 847],
    [ 563, 562, 557, 556, 551, 550, 545, 544,  77,  76,  -2,  33,   9,   8,  25,  24,  -1,  -2,  71,  70, 119, 118, 539, 538, 533, 532, 527, 526, 849,  -3],
    [ 565, 564, 571, 570, 577, 576, 583, 582, 589, 588, 595, 594, 601, 600, 607, 606, 613, 612, 619, 618, 625, 624, 631, 630, 637, 636, 643, 642, 851, 850],
    [ 567, 566, 573, 572, 579, 578, 585, 584, 591, 590, 597, 596, 603, 602, 609, 608, 615, 614, 621, 620, 627, 626, 633, 632, 639, 638, 645, 644, 852,  -3],
    [ 569, 568, 575, 574, 581, 580, 587, 586, 593, 592, 599, 598, 605, 604, 611, 610, 617, 616, 623, 622, 629, 628, 635, 634, 641, 640, 647, 646, 854, 853],
    [ 727, 726, 721, 720, 715, 714, 709, 708, 703, 702, 697, 696, 691, 690, 685, 684, 679, 678, 673, 672, 667, 666, 661, 660, 655, 654, 649, 648, 855,  -3],
    [ 729, 728, 723, 722, 717, 716, 711, 710, 705, 704, 699, 698, 693, 692, 687, 686, 681, 680, 675, 674, 669, 668, 663, 662, 657, 656, 651, 650, 857, 856],
    [ 731, 730, 725, 724, 719, 718, 713, 712, 707, 706, 701, 700, 695, 694, 689, 688, 683, 682, 677, 676, 671, 670, 665, 664, 659, 658, 653, 652, 858,  -3],
    [ 733, 732, 739, 738, 745, 744, 751, 750, 757, 756, 763, 762, 769, 768, 775, 774, 781, 780, 787, 786, 793, 792, 799, 798, 805, 804, 811, 810, 860, 859],
    [ 735, 734, 741, 740, 747, 746, 753, 752, 759, 758, 765, 764, 771, 770, 777, 776, 783, 782, 789, 788, 795, 794, 801, 800, 807, 806, 813, 812, 861,  -3],
    [ 737, 736, 743, 742, 749, 748, 755, 754, 761, 760, 767, 766, 773, 772, 779, 778, 785, 784, 791, 790, 797, 796, 803, 802, 809, 808, 815, 814, 863, 862],
];

// ---------------------------------------------------------------------------
// Reed-Solomon encoding over GF(64), ported from libzint reedsol.c
// ---------------------------------------------------------------------------

/// Compute generator polynomial coefficients for `nsym` EC symbols.
/// Uses roots alpha^1, alpha^2, ..., alpha^nsym (index=1 convention).
fn rs_build_generator(nsym: usize) -> Vec<u8> {
    let mut poly = vec![0u8; nsym + 1];
    poly[0] = 1;
    for i in 1..=nsym {
        poly[i] = 1;
        for k in (1..i).rev() {
            if poly[k] != 0 {
                poly[k] = ALOG[LOGT[poly[k] as usize] as usize + i];
            }
            poly[k] ^= poly[k - 1];
        }
        poly[0] = ALOG[LOGT[poly[0] as usize] as usize + i];
    }
    poly
}

/// Reed-Solomon encode: given data codewords, compute `nsym` EC codewords.
/// Result is written to `res[0..nsym]` (reversed at end, per libzint).
fn rs_encode(data: &[u8], nsym: usize, gen: &[u8], res: &mut [u8]) {
    let n = nsym;
    for v in res[..n].iter_mut() {
        *v = 0;
    }
    for &d in data {
        let m = res[n - 1] ^ d;
        if m != 0 {
            let log_m = LOGT[m as usize] as usize;
            for k in (1..n).rev() {
                res[k] = res[k - 1] ^ ALOG[log_m + LOGT[gen[k] as usize] as usize];
            }
            res[0] = ALOG[log_m + LOGT[gen[0] as usize] as usize];
        } else {
            res.copy_within(0..n - 1, 1);
            res[0] = 0;
        }
    }
    res[..n].reverse();
}

// ---------------------------------------------------------------------------
// Primary message encoding (ported from libzint mx_do_primary_2/3)
// ---------------------------------------------------------------------------

fn encode_primary_mode2(codewords: &mut [u8; 144], postcode: &[u8], country: u32, service: u32) {
    let postal_bytes: Vec<u8> = postcode
        .iter()
        .copied()
        .take_while(|&b| b != b' ')
        .collect();
    let postcode_len = postal_bytes.len();
    let postcode_num: u32 = postal_bytes
        .iter()
        .fold(0u32, |acc, &b| acc * 10 + (b - b'0') as u32);
    codewords[0] = (((postcode_num & 0x03) << 4) | 2) as u8;
    codewords[1] = ((postcode_num & 0xFC) >> 2) as u8;
    codewords[2] = ((postcode_num & 0x3F00) >> 8) as u8;
    codewords[3] = ((postcode_num & 0xFC000) >> 14) as u8;
    codewords[4] = ((postcode_num & 0x3F00000) >> 20) as u8;
    codewords[5] =
        (((postcode_num & 0x3C000000) >> 26) | (((postcode_len as u32) & 0x03) << 4)) as u8;
    codewords[6] = ((((postcode_len as u32) & 0x3C) >> 2) | ((country & 0x03) << 4)) as u8;
    codewords[7] = ((country & 0xFC) >> 2) as u8;
    codewords[8] = (((country & 0x300) >> 8) | ((service & 0x0F) << 2)) as u8;
    codewords[9] = ((service & 0x3F0) >> 4) as u8;
}

fn encode_primary_mode3(
    codewords: &mut [u8; 144],
    postcode6: &[u8; 6],
    country: u32,
    service: u32,
) {
    let mut pc = [0u8; 6];
    for i in 0..6 {
        let ch = postcode6[i];
        let ch_up = if ch.is_ascii_lowercase() { ch - 32 } else { ch };
        pc[i] = MAXI_SYMBOL_CHAR[ch_up as usize];
    }
    codewords[0] = ((pc[5] & 0x03) << 4) | 3;
    codewords[1] = ((pc[4] & 0x03) << 4) | ((pc[5] & 0x3C) >> 2);
    codewords[2] = ((pc[3] & 0x03) << 4) | ((pc[4] & 0x3C) >> 2);
    codewords[3] = ((pc[2] & 0x03) << 4) | ((pc[3] & 0x3C) >> 2);
    codewords[4] = ((pc[1] & 0x03) << 4) | ((pc[2] & 0x3C) >> 2);
    codewords[5] = ((pc[0] & 0x03) << 4) | ((pc[1] & 0x3C) >> 2);
    codewords[6] = (((pc[0] as u32 & 0x3C) >> 2) | ((country & 0x03) << 4)) as u8;
    codewords[7] = ((country & 0xFC) >> 2) as u8;
    codewords[8] = (((country & 0x300) >> 8) | ((service & 0x0F) << 2)) as u8;
    codewords[9] = ((service & 0x3F0) >> 4) as u8;
}

// ---------------------------------------------------------------------------
// Primary error correction (10 data + 10 EC)
// ---------------------------------------------------------------------------

fn apply_primary_ecc(codewords: &mut [u8; 144]) {
    let gen = rs_build_generator(10);
    let mut ec = [0u8; 10];
    rs_encode(&codewords[0..10], 10, &gen, &mut ec);
    codewords[10..20].copy_from_slice(&ec);
}

// ---------------------------------------------------------------------------
// Secondary message encoding
//
// The five-state shortest-path encoder below is an idiomatic Rust adaptation
// of libzint's MaxiCode text compaction algorithm (BSD-3-Clause, Copyright
// 2010-2026 Robin Stuart). It considers all sets A-E, every legal shift/latch
// transition and the 9-digit numeric shift instead of making local greedy
// choices.
// ---------------------------------------------------------------------------

const SET_A: usize = 0;
const SET_B: usize = 1;
const SET_E: usize = 2;
const SET_C: usize = 3;
const SET_D: usize = 4;
const SET_COUNT: usize = 5;
const SET_FLAGS: [u8; SET_COUNT] = [0x01, 0x02, 0x04, 0x08, 0x10];
const PAD_CODEWORD: u8 = 33;

#[derive(Clone, Copy)]
enum TextOperation {
    Direct(usize),
    Shift(usize),
    ShiftA(usize),
}

const OPS_A: [TextOperation; 5] = [
    TextOperation::Direct(SET_A),
    TextOperation::Shift(SET_B),
    TextOperation::Shift(SET_E),
    TextOperation::Shift(SET_C),
    TextOperation::Shift(SET_D),
];
const OPS_B: [TextOperation; 7] = [
    TextOperation::Direct(SET_B),
    TextOperation::ShiftA(1),
    TextOperation::ShiftA(2),
    TextOperation::ShiftA(3),
    TextOperation::Shift(SET_E),
    TextOperation::Shift(SET_C),
    TextOperation::Shift(SET_D),
];
const OPS_E: [TextOperation; 3] = [
    TextOperation::Direct(SET_E),
    TextOperation::Shift(SET_C),
    TextOperation::Shift(SET_D),
];
const OPS_C: [TextOperation; 3] = [
    TextOperation::Direct(SET_C),
    TextOperation::Shift(SET_E),
    TextOperation::Shift(SET_D),
];
const OPS_D: [TextOperation; 3] = [
    TextOperation::Direct(SET_D),
    TextOperation::Shift(SET_E),
    TextOperation::Shift(SET_C),
];

fn operations(set: usize) -> &'static [TextOperation] {
    match set {
        SET_A => &OPS_A,
        SET_B => &OPS_B,
        SET_E => &OPS_E,
        SET_C => &OPS_C,
        SET_D => &OPS_D,
        _ => unreachable!("invalid MaxiCode set"),
    }
}

/// Latch from `from` to `to`. Rows in libzint's table describe the later set,
/// columns the prior set, hence the seemingly asymmetric A/E and A/C paths.
fn latch_sequence(from: usize, to: usize) -> &'static [u8] {
    if from == to {
        return &[];
    }
    match to {
        SET_A => match from {
            SET_B => &[63],
            _ => &[58],
        },
        SET_B => &[63],
        SET_E => &[62, 62],
        SET_C => &[60, 60],
        SET_D => &[61, 61],
        _ => unreachable!("invalid MaxiCode set"),
    }
}

fn symbol_for_set(set: usize, ch: u8) -> u8 {
    let flags = MAXI_CODE_SET[ch as usize];
    if flags == SET_FLAGS[set] || set == SET_A {
        return MAXI_SYMBOL_CHAR[ch as usize];
    }
    if set == SET_B {
        if let Some(position) = b" ,./:".iter().position(|value| *value == ch) {
            return 47 + position as u8;
        }
    }
    if set == SET_E && (28..=30).contains(&ch) {
        return ch + 4;
    }
    if ch == b' ' {
        59
    } else {
        ch
    }
}

fn operation_codewords(operation: TextOperation, msg: &[u8], position: usize) -> Option<Vec<u8>> {
    match operation {
        TextOperation::Direct(set) => {
            let ch = *msg.get(position)?;
            (MAXI_CODE_SET[ch as usize] & SET_FLAGS[set] != 0)
                .then(|| vec![symbol_for_set(set, ch)])
        }
        TextOperation::Shift(set) => {
            let ch = *msg.get(position)?;
            if MAXI_CODE_SET[ch as usize] & SET_FLAGS[set] == 0 {
                return None;
            }
            let shift = match set {
                SET_A | SET_B => 59,
                SET_C => 60,
                SET_D => 61,
                SET_E => 62,
                _ => unreachable!("invalid MaxiCode set"),
            };
            Some(vec![shift, symbol_for_set(set, ch)])
        }
        TextOperation::ShiftA(count) => {
            let chars = msg.get(position..position + count)?;
            if !chars
                .iter()
                .all(|ch| MAXI_CODE_SET[*ch as usize] & SET_FLAGS[SET_A] != 0)
            {
                return None;
            }
            let mut result = Vec::with_capacity(count + 1);
            result.push(match count {
                1 => 59,
                2 => 56,
                3 => 57,
                _ => unreachable!("invalid Set A shift length"),
            });
            result.extend(chars.iter().map(|ch| symbol_for_set(SET_A, *ch)));
            Some(result)
        }
    }
}

fn operation_intake(operation: TextOperation) -> usize {
    match operation {
        TextOperation::ShiftA(count) => count,
        TextOperation::Direct(_) | TextOperation::Shift(_) => 1,
    }
}

fn numeric_codewords(digits: &[u8]) -> Option<[u8; 6]> {
    if digits.len() < 9 || !digits[..9].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let value = digits[..9]
        .iter()
        .fold(0u32, |value, digit| value * 10 + u32::from(*digit - b'0'));
    Some([
        31,
        ((value >> 24) & 0x3f) as u8,
        ((value >> 18) & 0x3f) as u8,
        ((value >> 12) & 0x3f) as u8,
        ((value >> 6) & 0x3f) as u8,
        (value & 0x3f) as u8,
    ])
}

fn update_shortest(slot: &mut Option<Vec<u8>>, candidate: Vec<u8>) {
    if slot
        .as_ref()
        .is_none_or(|current| candidate.len() < current.len())
    {
        *slot = Some(candidate);
    }
}

/// Encode message bytes into exactly `max_slots` data codewords.
fn encode_message(msg: &[u8], max_slots: usize) -> Result<Vec<u8>, String> {
    // Numeric shift is the densest MaxiCode representation: nine input bytes
    // in six codewords. Reject impossible lengths before allocating the path
    // table so hostile input cannot turn this small optimizer into a memory
    // sink.
    let maximum_input = (max_slots / 6) * 9 + (max_slots % 6);
    if msg.len() > maximum_input {
        return Err(format!(
            "MaxiCode message exceeds the {max_slots}-codeword capacity"
        ));
    }

    let mut paths = vec![vec![None::<Vec<u8>>; SET_COUNT]; msg.len() + 1];
    paths[0][SET_A] = Some(Vec::new());

    for position in 0..msg.len() {
        for from_set in 0..SET_COUNT {
            let Some(prefix) = paths[position][from_set].clone() else {
                continue;
            };

            for (to_set, _) in SET_FLAGS.iter().enumerate() {
                let latch = latch_sequence(from_set, to_set);

                // Nine digits always fit in fewer codewords than any text-set
                // representation, independent of the active set.
                if let Some(numeric) = numeric_codewords(&msg[position..]) {
                    let mut candidate = prefix.clone();
                    candidate.extend_from_slice(latch);
                    candidate.extend_from_slice(&numeric);
                    update_shortest(&mut paths[position + 9][to_set], candidate);
                    continue;
                }

                for &operation in operations(to_set) {
                    let Some(encoded) = operation_codewords(operation, msg, position) else {
                        continue;
                    };
                    let next_position = position + operation_intake(operation);
                    let mut candidate = prefix.clone();
                    candidate.extend_from_slice(latch);
                    candidate.extend_from_slice(&encoded);
                    update_shortest(&mut paths[next_position][to_set], candidate);
                }
            }
        }
    }

    let (end_set, mut out) = paths[msg.len()]
        .iter()
        .enumerate()
        .filter_map(|(set, path)| path.clone().map(|path| (set, path)))
        .min_by_key(|(_, path)| path.len())
        .ok_or_else(|| {
            let ch = msg
                .iter()
                .find(|ch| MAXI_CODE_SET[**ch as usize] == 0)
                .copied()
                .unwrap_or_default();
            format!("MaxiCode cannot encode byte 0x{ch:02X}")
        })?;

    if out.len() > max_slots {
        return Err(format!(
            "MaxiCode message exceeds the {max_slots}-codeword capacity"
        ));
    }

    // Sets C and D have no PAD character. Return to A before padding when
    // there is room, exactly as the reference encoder does.
    if out.len() < max_slots && matches!(end_set, SET_C | SET_D) {
        out.push(58);
    }
    if out.len() > max_slots {
        return Err(format!(
            "MaxiCode message exceeds the {max_slots}-codeword capacity"
        ));
    }

    let pad = if end_set == SET_E { 28 } else { PAD_CODEWORD };
    out.resize(max_slots, pad);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Secondary error correction
// 84 data codewords (20..104), split EVEN/ODD; each half 42 data + 20 EC.
// EC placed at 104..144 (interleaved even/odd).
// ---------------------------------------------------------------------------

fn apply_secondary_ecc(codewords: &mut [u8; 144]) {
    let gen = rs_build_generator(20);

    // Even indices: 20, 22, ..., 102  (42 data codewords)
    let mut data_even = [0u8; 42];
    for j in 0..42usize {
        data_even[j] = codewords[20 + j * 2];
    }
    let mut ec_even = [0u8; 20];
    rs_encode(&data_even, 20, &gen, &mut ec_even);
    for j in 0..20usize {
        codewords[104 + j * 2] = ec_even[j];
    }

    // Odd indices: 21, 23, ..., 103  (42 data codewords)
    let mut data_odd = [0u8; 42];
    for j in 0..42usize {
        data_odd[j] = codewords[21 + j * 2];
    }
    let mut ec_odd = [0u8; 20];
    rs_encode(&data_odd, 20, &gen, &mut ec_odd);
    for j in 0..20usize {
        codewords[105 + j * 2] = ec_odd[j];
    }
}

// ---------------------------------------------------------------------------
// Public encode entry point
// ---------------------------------------------------------------------------

fn latin1_bytes(data: &str) -> Result<Vec<u8>, String> {
    data.chars()
        .map(|ch| {
            u8::try_from(ch as u32)
                .map_err(|_| format!("MaxiCode cannot encode character U+{:04X}", ch as u32))
        })
        .collect()
}

/// Encode a MaxiCode symbol into its 144 six-bit codewords.
///
/// `data`  — raw ZPL `^FD` field content (hex escapes already resolved by the parser).
/// `mode`  — MaxiCode mode from the `^BD` command (typically 2, 3, or 4).
///
/// For mode 2/3 the ZPL convention is:
///   data = {service(3)}{country(3)}{postal}[)>{secondary}
/// For mode 4 the entire `data` string is the secondary message.
pub fn encode_codewords(data: &str, mode: i32) -> Result<[u8; 144], String> {
    if data.is_empty() {
        return Err("MaxiCode: empty content".to_string());
    }

    let mut codewords = [0u8; 144];

    match mode {
        2 | 3 => {
            // Split primary (before "[)>") from secondary (from "[)>" onwards)
            let split_pos = data.find("[)>").unwrap_or(data.len());
            let primary_str = &data[..split_pos];
            let secondary_str = &data[split_pos..];

            if primary_str.len() < 7 {
                return Err(format!(
                    "MaxiCode mode {mode}: primary too short ({} chars, need >=7)",
                    primary_str.len()
                ));
            }

            // ZPL primary format: service(3) + country(3) + postal.
            // Validate bytes before splitting so non-ASCII input cannot panic
            // at an invalid UTF-8 boundary.
            let primary_bytes = primary_str.as_bytes();
            if !primary_bytes[..3].iter().all(u8::is_ascii_digit) {
                return Err("MaxiCode service class must contain 3 digits".to_string());
            }
            if !primary_bytes[3..6].iter().all(u8::is_ascii_digit) {
                return Err("MaxiCode country code must contain 3 digits".to_string());
            }
            let parse_digits = |digits: &[u8]| {
                digits
                    .iter()
                    .fold(0u32, |value, digit| value * 10 + u32::from(*digit - b'0'))
            };
            let service = parse_digits(&primary_bytes[..3]);
            let country = parse_digits(&primary_bytes[3..6]);
            let postal_bytes = &primary_bytes[6..];

            if mode == 2 {
                if postal_bytes.is_empty()
                    || postal_bytes.len() > 9
                    || !postal_bytes.iter().all(u8::is_ascii_digit)
                {
                    return Err(
                        "MaxiCode mode 2 postal code must contain 1 to 9 digits".to_string()
                    );
                }
                encode_primary_mode2(&mut codewords, postal_bytes, country, service);
            } else {
                if postal_bytes.len() != 6
                    || !postal_bytes
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b' ')
                {
                    return Err(
                        "MaxiCode mode 3 postal code must contain exactly 6 alphanumeric characters"
                            .to_string(),
                    );
                }
                let pc6: [u8; 6] = postal_bytes
                    .try_into()
                    .expect("validated six-byte postal code");
                encode_primary_mode3(&mut codewords, &pc6, country, service);
            }

            let secondary_bytes = latin1_bytes(secondary_str)?;
            let message = encode_message(&secondary_bytes, 84)?;
            codewords[20..104].copy_from_slice(&message);
        }
        4 => {
            codewords[0] = 4;
            // Mode 4 has 93 message codewords. The first nine share the
            // primary data block with the mode codeword; the remaining 84
            // occupy the secondary data block.
            let data_bytes = latin1_bytes(data)?;
            let message = encode_message(&data_bytes, 93)?;
            codewords[1..10].copy_from_slice(&message[..9]);
            codewords[20..104].copy_from_slice(&message[9..]);
        }
        _ => {
            return Err(format!(
                "MaxiCode mode {mode} is not supported; expected 2, 3, or 4"
            ));
        }
    }

    apply_primary_ecc(&mut codewords);
    apply_secondary_ecc(&mut codewords);

    Ok(codewords)
}

/// Encode a MaxiCode symbol and return a 200x193 RGBA image.
pub fn encode(data: &str, mode: i32) -> Result<RgbaImage, String> {
    let codewords = encode_codewords(data, mode)?;

    // --- Render -----------------------------------------------------------
    let img_width = 200u32;
    let img_height = 193u32;
    let col_pitch = img_width as f32 / 30.0;
    let row_pitch = img_height as f32 / 33.0;

    let mut img = RgbaImage::from_pixel(img_width, img_height, Rgba([0, 0, 0, 0]));
    let black = Rgba([0u8, 0, 0, 255]);

    // Bullseye rings (radii calibrated to Labelary reference)
    let cx = img_width / 2;
    let cy = img_height / 2;
    draw_ring(&mut img, cx, cy, 4, 9, black);
    draw_ring(&mut img, cx, cy, 14, 19, black);
    draw_ring(&mut img, cx, cy, 24, 29, black);

    // Hex module grid
    for (row_idx, bitnr_row) in BITNR.iter().enumerate() {
        let x_half_off = if row_idx % 2 == 1 {
            col_pitch / 2.0
        } else {
            0.0
        };
        for (col_idx, &bit_nr) in bitnr_row.iter().enumerate() {
            let px = (col_idx as f32 * col_pitch + x_half_off + col_pitch / 2.0).round() as u32;
            let py = (row_idx as f32 * row_pitch + row_pitch / 2.0).round() as u32;
            match bit_nr {
                -3 => {}                                        // bullseye area — skip
                -1 => {}                                        // always white
                -2 => draw_hexagon(&mut img, px, py, 3, black), // orientation mark
                n if n >= 0 => {
                    let n = n as usize;
                    if n < 864 {
                        let cw = codewords[n / 6];
                        let bit_pos = 5 - (n % 6);
                        if (cw >> bit_pos) & 1 != 0 {
                            draw_hexagon(&mut img, px, py, 3, black);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(img)
}

fn draw_ring(img: &mut RgbaImage, cx: u32, cy: u32, inner_r: u32, outer_r: u32, color: Rgba<u8>) {
    let r2_outer = (outer_r * outer_r) as i64;
    let r2_inner = (inner_r * inner_r) as i64;
    let d = outer_r as i32;
    for dy in -d..=d {
        for dx in -d..=d {
            let dist2 = dx as i64 * dx as i64 + dy as i64 * dy as i64;
            if dist2 >= r2_inner && dist2 <= r2_outer {
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

fn draw_hexagon(img: &mut RgbaImage, cx: u32, cy: u32, r: u32, color: Rgba<u8>) {
    let r2 = (r * r) as i64;
    let ri = r as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            if (dx as i64 * dx as i64 + dy as i64 * dy as i64) <= r2 {
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{encode_codewords, encode_message};

    // Full 144-codeword vectors generated with libzint commit
    // 6ac010af0e9f1caebc06c1941b84f1904204cad6 in DATA_MODE, MaxiCode mode 4,
    // with ZINT_DEBUG_PRINT. These exercise the public encoder including both
    // Reed-Solomon blocks, rather than only testing internal compaction output.
    const LIBZINT_SET_C: [u8; 144] = [
        4, 60, 60, 0, 0, 0, 28, 29, 30, 59, 60, 5, 57, 7, 24, 21, 37, 54, 40, 17, 58, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 8, 60, 19, 40, 20, 9, 9, 43, 1, 14, 3, 50, 8,
        12, 32, 53, 28, 57, 16, 58, 11, 36, 23, 28, 56, 10, 18, 53, 44, 37, 19, 30, 48, 14, 33, 5,
        39, 31, 39, 40,
    ];
    const LIBZINT_SET_D: [u8; 144] = [
        4, 61, 61, 0, 0, 0, 28, 29, 30, 59, 6, 38, 35, 25, 50, 30, 12, 26, 5, 17, 58, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 8, 60, 19, 40, 20, 9, 9, 43, 1, 14, 3, 50, 8,
        12, 32, 53, 28, 57, 16, 58, 11, 36, 23, 28, 56, 10, 18, 53, 44, 37, 19, 30, 48, 14, 33, 5,
        39, 31, 39, 40,
    ];
    const LIBZINT_SET_E: [u8; 144] = [
        4, 62, 62, 1, 2, 3, 13, 32, 33, 34, 61, 15, 49, 58, 12, 12, 52, 38, 39, 49, 59, 28, 28, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 39, 26, 30, 45, 5, 49, 2, 38, 17, 31, 3, 5, 15,
        44, 26, 43, 40, 7, 45, 12, 27, 1, 19, 62, 20, 58, 52, 43, 36, 57, 51, 13, 23, 31, 42, 29,
        46, 53, 35, 45,
    ];
    const LIBZINT_MIXED: [u8; 144] = [
        4, 1, 31, 7, 22, 60, 52, 21, 63, 2, 16, 27, 40, 34, 32, 50, 55, 12, 11, 42, 31, 7, 22, 60,
        52, 21, 2, 2, 2, 63, 1, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 33,
        33, 33, 33, 33, 33, 33, 33, 33, 33, 33, 55, 56, 56, 45, 53, 41, 57, 9, 17, 61, 9, 41, 13,
        12, 3, 23, 10, 25, 23, 42, 54, 28, 13, 1, 20, 36, 43, 17, 24, 60, 6, 50, 50, 29, 36, 49,
        11, 43, 19, 33,
    ];

    #[test]
    fn public_encoder_matches_libzint_sets_c_d_e_and_mixed_vectors() {
        let cases: [(&str, &[u8; 144]); 4] = [
            ("ÀÀÀ\u{1C}\u{1D}\u{1E} ", &LIBZINT_SET_C),
            ("ààà\u{1C}\u{1D}\u{1E} ", &LIBZINT_SET_D),
            ("\u{01}\u{02}\u{03}\r\u{1C}\u{1D}\u{1E} ", &LIBZINT_SET_E),
            ("A123456789b123456789bbbA", &LIBZINT_MIXED),
        ];
        for (input, expected) in cases {
            assert_eq!(&encode_codewords(input, 4).unwrap(), expected);
        }
    }

    #[test]
    fn public_encoder_rejects_characters_outside_latin1() {
        assert!(encode_codewords("€", 4).is_err());
    }

    #[test]
    fn text_encoder_reaches_all_five_code_sets() {
        assert_eq!(&encode_message(b"A", 4).unwrap()[..1], &[1]);
        assert_eq!(&encode_message(b"a", 4).unwrap()[..2], &[59, 1]);
        assert_eq!(&encode_message(&[0x00], 4).unwrap()[..2], &[62, 0]);
        assert_eq!(&encode_message(&[0xC0], 4).unwrap()[..2], &[60, 0]);
        assert_eq!(&encode_message(&[0xE0], 4).unwrap()[..2], &[61, 0]);
    }

    #[test]
    fn text_encoder_chooses_latch_for_lowercase_run() {
        assert_eq!(&encode_message(b"abcd", 8).unwrap()[..5], &[63, 1, 2, 3, 4]);
    }

    #[test]
    fn text_encoder_compacts_nine_digits_into_six_codewords() {
        assert_eq!(
            &encode_message(b"123456789", 10).unwrap()[..6],
            &[31, 7, 22, 60, 52, 21]
        );
    }
}
