/// Calculates the key constants, mu, and reciprocal polynomial values required for
/// carryless-multiplication computation as seen in the "Fast CRC Computation for Generic
/// Polynomials Using PCLMULQDQ Instruction" white paper from Intel.
///
/// Tested against the ECMA-182 (0x42F0E1EBA9EA3693) as used in CRC-64/XZ and
/// NVME/Rocksoft (0xAD93D23594C93659) as used in CRC-64/NVME polynomials.
///
/// Derived from: https://github.com/jeffareid/crc/blob/master/crc64r/crc64rg.cpp
/// With help from: https://github.com/intel/isa-l/issues/88
///
/// Stackoverflow insights:
/// https://stackoverflow.com/questions/71328336/fast-crc-with-pclmulqdq-not-reflected/71329114#71329114
/// https://stackoverflow.com/questions/21171733/calculating-constants-for-crc32-using-pclmulqdq
///
/// Linux's implementations: https://github.com/torvalds/linux/blob/786c8248dbd33a5a7a07f7c6e55a7bfc68d2ca48/lib/crc64.c
///
/// [Intel white paper]: https://web.archive.org/web/20131224125630/https://www.intel.com/content/dam/www/public/us/en/documents/white-papers/fast-crc-computation-generic-polynomials-pclmulqdq-paper.pdf

extern crate core;

use std::env;

// the key sizes to calculate, given this is a CRC-64 (rather than a CRC-32, as in the Intel paper)
static KEY_SIZES: [u32; 16] = [
    128,
    192,
    256,
    320,
    384,
    448,
    512,
    576,
    640,
    704,
    768,
    832,
    896,
    960,
    1024,
    1088,
];

/// Reverses the bits of a 64-bit unsigned integer.
///
/// This function iterates over each bit of the input `u64` value, `f`, from the least significant
/// bit to the most significant bit,  reversing its order. The reversed bit order is accumulated
/// in `r` and returned. This operation is commonly used in bit manipulation
/// tasks such as computing reverse CRCs or working with binary protocols.
///
/// # Parameters
///
/// * `forward`: The 64-bit unsigned integer whose bits are to be reversed.
///
/// # Returns
///
/// * A `u64` value representing the bit-reversed version of `forward`.
///
/// # Examples
///
/// ```
/// let original = 0b0000001000000000000000000000000000000000000000000000000000000000;
/// let reversed = bit_reverse(original);
/// assert_eq!(reversed, 0b0000000000000000000000000000000000000000000000000000001000000000);
/// ```
///
/// (Docs generated by GitHub Copilot)
fn bit_reverse(mut forward: u64) -> u64 {
    let mut reversed = 0;

    for _ in 0..64 {
        reversed <<= 1;
        reversed |= forward & 1;
        forward >>= 1;
    }

    reversed
}

/// Generates the multiplicative inverse (\mu) for a given polynomial.
///
/// This function calculates the multiplicative inverse (\mu) of a given polynomial, which is
/// used in the Barrett reduction for optimizing the division operation in polynomial arithmetic,
/// particularly in CRC calculations. The calculation follows the method described in the Intel
/// white paper on fast CRC computation using the PCLMULQDQ instruction.
///
/// The process involves iteratively shifting and XORing values to simulate polynomial division,
/// with the result being  bit-reversed at the end to obtain the final \mu value. This value is
/// essential for efficiently computing CRC values using Barrett reduction.
///
/// # Parameters
///
/// * `polynomial`: The polynomial for which the multiplicative inverse is to be calculated.
///                 This is typically the CRC polynomial.
///
/// # Returns
///
/// * The multiplicative inverse (\mu) of the given polynomial as a `u64`.
///
/// # Example
///
/// ```
/// let poly = 0xAD93D23594C93659; // CRC-64-NVME polynomial
/// let mu = generate_mu(poly);
/// println!("The multiplicative inverse (mu) for the given polynomial is: {:X}", mu);
/// ```
///
/// (Docs generated by GitHub Copilot)
fn generate_mu(polynomial: u64) -> u64 {
    // High part of the numerator, initialized to 1 for division.
    let mut numerator_high = 0x0000000000000001;

    // Low part of the numerator, starts at 0.
    let mut numerator_low = 0x0000000000000000;

    // The quotient, initialized to 0.
    let mut quotient = 0;

    for _ in 0..64 {
        // Shift the quotient left by 1 bit to make room for the next bit.
        quotient <<= 1;

        if numerator_high != 0 {
            // Set the least significant bit of Q if Nhi is not 0.
            quotient |= 1;

            // Perform the XOR operation as part of the division.
            numerator_low ^= polynomial;
        }
        // Update Nhi to the most significant bit of Nlo.
        numerator_high = numerator_low >> 63;

        // Shift Nlo left by 1 bit for the next iteration.
        numerator_low <<= 1;
    }

    // Bit-reverse the quotient to get the final \(\mu\) constant.
    bit_reverse(quotient)
}

/// Generates a key for a given polynomial and exponent.
///
/// This function computes a key for polynomial-based operations, such as CRC calculations,
/// using a specified polynomial and exponent. The key generation involves bit manipulation
/// and arithmetic operations that simulate the polynomial division process. The result is
/// then bit-reversed to obtain the final key value. This function incorporates Rust's
/// `wrapping_sub` method to safely handle underflow conditions that can occur during the
/// subtraction operation.
///
/// # Parameters
///
/// * `exponent`: The exponent value, representing the degree to which the polynomial is raised.
///               If `exponent` is less than or equal to 64, the function returns 0, as the
///               operation does not produce a meaningful result in such cases.
/// * `polynomial`: The polynomial used for the key generation. This is typically a CRC polynomial.
///
/// # Returns
///
/// * A `u64` representing the generated key, which is the bit-reversed result of the
///   polynomial division simulation.
///
/// # Examples
///
/// ```
/// let poly = 0xAD93D23594C93659; // CRC-64-NVME polynomial
/// let exponent = 128;
/// let key = generate_key(exponent, poly);
/// println!("Generated key: {:X}", key);
/// ```
///
/// (Docs generated by GitHub Copilot)
fn generate_key(exponent: u64, polynomial: u64) -> u64 {
    // Initialize N with the highest bit set.
    let mut n = 0x8000000000000000;

    if exponent <= 64 {
        // Return 0 for exponents 64 or less, as no key is needed.
        return 0;
    }

    // Adjust exponent to fit 64-bit operation.
    let e = exponent - 64;

    for _ in 0..e {
        // Shift and XOR if the highest bit is set.
        n = (n << 1) ^ ((0x00u64.wrapping_sub(n >> 63)) & polynomial);
    }

    // Bit-reverse the result to match reflected CRC-64 requirements.
    bit_reverse(n)
}

/// Generates the reciprocal polynomial for a given polynomial.
///
/// This function calculates the reciprocal polynomial by first reversing the bits of the input
/// polynomial using the `bit_reverse` function. It then shifts the result to the left by one
/// position and sets the least significant bit to 1. The reciprocal polynomial is used in certain
/// CRC calculations and other polynomial arithmetic operations where the inverse representation
/// of a polynomial is required.
///
/// # Parameters
///
/// * `polynomial`: The polynomial for which the reciprocal is to be calculated.
///
/// # Returns
///
/// * The reciprocal polynomial as a `u64`.
///
/// # Examples
///
/// ```
/// let poly = 0xAD93D23594C93659; // CRC-64-NVME polynomial
/// let reciprocal = generate_reciprocal_polynomial(poly);
/// println!("Reciprocal polynomial: {:X}", reciprocal);
/// ```
///
/// (Docs generated by GitHub Copilot)
fn generate_reciprocal_polynomial(polynomial: u64) -> u64 {
    (bit_reverse(polynomial) << 1) | 1
}

/// Entry point of the application.
///
/// This function parses command-line arguments to extract a polynomial value, then calculates and displays
/// key constants, the multiplicative inverse (\mu), and the reciprocal polynomial for the given polynomial.
/// It demonstrates the use of various functions defined in this module to perform these calculations.
///
/// The polynomial is expected to be provided in hexadecimal format as a command-line argument.
/// If no polynomial is provided, the program prints usage information and exits.
///
/// # Examples
///
/// Running the program without arguments:
/// ```
/// cargo run
/// ```
/// This will output the usage information.
///
/// Running the program with a polynomial argument:
/// ```
/// cargo run 0xAD93D23594C93659
/// ```
/// This will calculate and display the key constants, \mu, and reciprocal polynomial for the given polynomial.
///
/// (Docs generated by GitHub Copilot)
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        println!("Usage: {} [POLYNOMIAL_HEX]", args[0]);
        return;
    }

    let polynomial = u64::from_str_radix(
        &args[1].trim_start_matches("0x"), 16
    ).expect("Failed to parse polynomial");

    for (_, &size) in KEY_SIZES.iter().enumerate() {
        println!("k_{} = 0x{:x}", size, generate_key(size as u64, polynomial));
    }

    println!("mu = 0x{:x}", generate_mu(polynomial));
    println!("reciprocal = 0x{:x}", generate_reciprocal_polynomial(polynomial));
}

#[cfg(test)]
mod bit_reverse_tests {
    use super::*;

    #[test]
    fn reverses_all_zeros_to_zeros() {
        assert_eq!(bit_reverse(0), 0);
    }

    #[test]
    fn reverses_all_ones_to_ones() {
        let all_ones: u64 = u64::MAX;
        assert_eq!(bit_reverse(all_ones), all_ones);
    }

    #[test]
    fn reverses_single_bit_at_start() {
        let input: u64 = 1 << 63; // Most significant bit set
        let expected: u64 = 1; // Least significant bit set
        assert_eq!(bit_reverse(input), expected);
    }

    #[test]
    fn reverses_single_bit_at_end() {
        let input: u64 = 1; // Least significant bit set
        let expected: u64 = 1 << 63; // Most significant bit set
        assert_eq!(bit_reverse(input), expected);
    }

    #[test]
    fn reverses_alternating_bits() {
        let input: u64 = 0b1010101010101010101010101010101010101010101010101010101010101010;
        let expected: u64 = 0b0101010101010101010101010101010101010101010101010101010101010101;
        assert_eq!(bit_reverse(input), expected);
    }

    #[test]
    fn reverses_example_polynomial() {
        let input: u64 = 0xAD93D23594C93659;
        let expected: u64 = 0x9a6c9329ac4bc9b5;
        let output: u64 = bit_reverse(input);

        println!("Expected {}", format!("{output:#x}"));

        assert_eq!(bit_reverse(input), expected);
    }
}

#[cfg(test)]
mod generate_mu_tests {
    use super::*;

    #[test]
    fn calculates_mu_for_known_crc64_ecma_polynomial() {
        let poly = 0x42F0E1EBA9EA3693; // Known CRC-64/XZ polynomial
        let expected_mu = 0x9c3e466c172963d5; // Expected mu for the given polynomial
        assert_eq!(generate_mu(poly), expected_mu);
    }

    #[test]
    fn calculates_mu_for_known_crc64_nvme_polynomial() {
        let poly = 0xAD93D23594C93659; // Known CRC-64/NVME polynomial
        let expected_mu = 0x27ecfa329aef9f77; // Expected mu for the given polynomial
        assert_eq!(generate_mu(poly), expected_mu);
    }
}

#[cfg(test)]
mod generate_key_tests {
    use super::*;

    #[test]
    fn generates_key_for_valid_exponent_and_polynomial_nvme() {
        static CASES: &[(u64, u64)] = &[
            (128, 0x21e9761e252621ac),
            (192, 0xeadc_41fd_2ba3_d420),
            (256, 0xe1e0_bb9d_45d7_a44c),
            (320, 0xb0bc_2e58_9204_f500),
            (384, 0xa3ff_dc1f_e8e8_2a8b),
            (448, 0xbdd7_ac0e_e1a4_a0f0),
            (512, 0x6224_2240_ace5_045a),
            (576, 0x0c32_cdb3_1e18_a84a),
            (640, 0x0336_3823_e6e7_91e5),
            (704, 0x7b0a_b10d_d0f8_09fe),
            (768, 0x34f5_a24e_22d6_6e90),
            (832, 0x3c25_5f5e_bc41_4423),
            (896, 0x9465_8840_3d4a_dcbc),
            (960, 0xd083_dd59_4d96_319d),
            (1024, 0x5f85_2fb6_1e8d_92dc),
            (1088, 0xa1ca681e733f9c40),
        ];

        let poly = 0xAD93D23594C93659; // Known CRC-64/NVME polynomial

        for (exponent, result) in CASES {
            assert_eq!(generate_key(*exponent, poly), *result);
        }
    }

    #[test]
    fn generates_key_for_valid_exponent_and_polynomial_ecma() {
        static CASES: &[(u64, u64)] = &[
            (128, 0xdabe_95af_c787_5f40),
            (192, 0xe05d_d497_ca39_3ae4),
            (256, 0x3be6_53a3_0fe1_af51),
            (320, 0x6009_5b00_8a9e_fa44),
            (384, 0x69a3_5d91_c373_0254),
            (448, 0xb5ea_1af9_c013_aca4),
            (512, 0x081f_6054_a784_2df4),
            (576, 0x6ae3_efbb_9dd4_41f3),
            (640, 0x0e31_d519_421a_63a5),
            (704, 0x2e30_2032_12ca_c325),
            (768, 0xe4ce_2cd5_5fea_0037),
            (832, 0x2fe3_fd29_20ce_82ec),
            (896, 0x9478_74de_5950_52cb),
            (960, 0x9e73_5cb5_9b47_24da),
            (1024, 0xd7d8_6b2a_f73d_e740),
            (1088, 0x8757_d71d_4fcc_1000),
        ];

        let poly = 0x42F0E1EBA9EA3693; // Known CRC-64/XZ ECMA-182 polynomial

        for (exponent, result) in CASES {
            assert_eq!(generate_key(*exponent, poly), *result);
        }
    }
}

#[cfg(test)]
mod generate_reciprocal_polynomial_tests {
    use super::*;

    #[test]
    fn reciprocal_of_specific_polynomial_nvme() {
        let poly = 0xAD93D23594C93659; // Known CRC-64/NVME polynomial
        let expected = 0x34d9_2653_5897_936b; // Expected reciprocal
        assert_eq!(generate_reciprocal_polynomial(poly), expected);
    }

    #[test]
    fn reciprocal_of_specific_polynomial_ecma() {
        let poly = 0x42F0E1EBA9EA3693; // Known CRC-64/XZ ECMA-182 polynomial
        let expected = 0x92d8_af2b_af0e_1e85; // Expected reciprocal
        assert_eq!(generate_reciprocal_polynomial(poly), expected);
    }
}
