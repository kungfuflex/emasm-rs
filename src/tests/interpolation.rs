use crate::*;

#[test]
fn test_single_placeholder() {
    let builder = evm_asm_interpolator!([
        &[0],
        0x40,
        "mstore",
        0x20,
        0x00,
        "return"
    ]);
    
    let bytecode = builder(Box::new(42u8));
    
    // Should have PUSH1 0x2a (42), PUSH1 0x40, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
    assert!(bytecode.contains(&0x2a)); // 42 in hex
    assert!(bytecode.contains(&0x40));
    assert!(bytecode.contains(&0x52)); // MSTORE
}

#[test]
fn test_multiple_placeholders() {
    let builder = evm_asm_interpolator!([
        &[0],
        0x40,
        "mstore",
        &[1],
        0x60,
        "mstore",
        0x80,
        0x00,
        "return"
    ]);
    
    let bytecode = builder(Box::new(10u128), Box::new(20u128));
    
    // Should contain both values
    assert!(bytecode.contains(&0x0a)); // 10
    assert!(bytecode.contains(&0x14)); // 20
}

#[test]
fn test_placeholder_with_large_value() {
    let builder = evm_asm_interpolator!([
        &[0],
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]);
    
    let large_value = 0x123456789abcdef0u64;
    let bytecode = builder(Box::new(large_value));
    
    // Should properly encode the large value as a PUSH instruction
    assert!(bytecode.len() > 10); // Multi-byte push + other ops
}

#[test]
fn test_placeholder_in_segment() {
    let builder = evm_asm_interpolator!([
        "start",
        "jump",
        ["start", [
            &[0],
            0x40,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]);
    
    let bytecode = builder(Box::new(99u8));
    
    // Should have JUMPDEST and the value 99
    assert!(bytecode.contains(&0x5b)); // JUMPDEST
    assert!(bytecode.contains(&0x63)); // 99 in hex
}
