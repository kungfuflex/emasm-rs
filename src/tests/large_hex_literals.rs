use crate::*;

#[test]
fn test_256bit_hex_literal() {
    // Test a 256-bit hex literal (common for EVM masks)
    // This is the mask for rounding down to 32-byte boundary: ~31 = 0xffffffe0 extended to 256 bits
    let bytecode = evm_asm!([
        0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0,
        "and"
    ]);

    // Should be PUSH32 followed by 32 bytes of data, then AND
    assert_eq!(bytecode[0], 0x7f); // PUSH32

    // The 32 bytes should be the hex value
    let expected_data = hex::decode("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0").unwrap();
    assert_eq!(&bytecode[1..33], &expected_data[..]);

    assert_eq!(bytecode[33], 0x16); // AND opcode
}

#[test]
fn test_256bit_hex_literal_in_interpolator() {
    // Test 256-bit hex literal in interpolator macro
    let builder = evm_asm_interpolator!([
        0x1f,
        "add",
        0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0,
        "and",
        &[0],  // placeholder for some value
        "add"
    ]);

    let bytecode = builder(Box::new(100u64));

    // Should compile without errors and produce valid bytecode
    assert!(bytecode.len() > 0);

    // Verify PUSH32 is present for the large constant
    assert!(bytecode.contains(&0x7f)); // PUSH32 opcode

    // Verify AND opcode is present
    assert!(bytecode.contains(&0x16)); // AND opcode
}

#[test]
fn test_mixed_size_literals() {
    // Test mixing small and large literals
    let bytecode = evm_asm!([
        0x20,                    // small - PUSH1
        0xffffffff,              // medium - PUSH4
        0xffffffffffffffff,      // larger - PUSH8
        0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff,  // max 256-bit
        "pop",
        "pop",
        "pop",
        "pop"
    ]);

    // Check that the bytecode starts with PUSH1 0x20
    assert_eq!(bytecode[0], 0x60); // PUSH1
    assert_eq!(bytecode[1], 0x20);

    // Should contain PUSH32 for the max value
    assert!(bytecode.contains(&0x7f)); // PUSH32
}

#[test]
fn test_256bit_not_mask() {
    // Another common pattern: NOT to create masks
    let bytecode = evm_asm!([
        0x1f,  // 31
        "not", // ~31 = 0xffff...ffe0
        "and"
    ]);

    // This should work as an alternative to the large literal
    assert!(bytecode.len() > 0);
    assert!(bytecode.contains(&0x19)); // NOT opcode
}

#[test]
fn test_segment_with_256bit_literal() {
    // Test 256-bit literal inside a segment
    let bytecode = evm_asm!([
        0x00,
        "start",
        "jump",
        ["start", [
            0x1f,
            "add",
            0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0,
            "and",
            0x00,
            "mstore"
        ]]
    ]);

    // Should compile and contain PUSH32
    assert!(bytecode.contains(&0x7f)); // PUSH32
    // Should contain JUMPDEST
    assert!(bytecode.contains(&0x5b)); // JUMPDEST
}
