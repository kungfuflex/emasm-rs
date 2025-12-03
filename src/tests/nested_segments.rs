use crate::*;

#[test]
fn test_deeply_nested_segments() {
    // Test segments nested 3 levels deep
    let bytecode = evm_asm!([
        0x00,
        "outer",
        "jump",
        ["outer", [
            0x01,
            "inner1",
            "jump",
            ["inner1", [
                0x02,
                "inner2",
                "jump",
                ["inner2", [
                    0x03,
                    "add",
                    0x00,
                    "mstore"
                ]]
            ]]
        ]]
    ]);

    // Should compile successfully with 3 levels of nesting
    assert!(bytecode.len() > 0);

    // Should contain 3 JUMPDESTs (one for each segment)
    let jumpdest_count = bytecode.iter().filter(|&&b| b == 0x5b).count();
    assert_eq!(jumpdest_count, 3, "Expected 3 JUMPDESTs for 3 nested segments");
}

#[test]
fn test_nested_segments_with_interpolator() {
    // Test nested segments in interpolator
    let builder = evm_asm_interpolator!([
        &[0],  // placeholder
        "outer",
        "jump",
        ["outer", [
            0x01,
            "inner",
            "jump",
            ["inner", [
                &[1],  // another placeholder
                "add",
                0x00,
                "mstore"
            ]]
        ]]
    ]);

    let bytecode = builder(Box::new(0x10u64), Box::new(0x20u64));

    // Should compile successfully
    assert!(bytecode.len() > 0);

    // Should contain 2 JUMPDESTs
    let jumpdest_count = bytecode.iter().filter(|&&b| b == 0x5b).count();
    assert_eq!(jumpdest_count, 2);
}

#[test]
fn test_sibling_nested_segments() {
    // Test multiple sibling segments at the same nesting level
    let bytecode = evm_asm!([
        0x00,
        "start",
        "jump",
        ["start", [
            0x01,
            "branch_a",
            "jumpi",
            "branch_b",
            "jump",
            ["branch_a", [
                0x0a,
                "add",
                "end",
                "jump"
            ]],
            ["branch_b", [
                0x0b,
                "add",
                "end",
                "jump"
            ]]
        ]],
        ["end", [
            0x00,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]);

    // Should compile successfully
    assert!(bytecode.len() > 0);

    // Should contain 4 JUMPDESTs (start, branch_a, branch_b, end)
    let jumpdest_count = bytecode.iter().filter(|&&b| b == 0x5b).count();
    assert_eq!(jumpdest_count, 4);
}

#[test]
fn test_copy_loop_pattern() {
    // This is similar to the pattern used in batch_payment_bytecode.rs
    let bytecode = evm_asm!([
        0x100,              // size
        0x40,               // src
        0x1000,             // dest
        "copy_loop",
        "jump",

        ["copy_loop", [
            // Stack: [dest, src, remaining]
            "dup3",         // [remaining, dest, src, remaining]
            "iszero",
            "copy_done",
            "jumpi",

            // Copy 32 bytes
            "dup2",         // [src, dest, src, remaining]
            "mload",        // [data, dest, src, remaining]
            "dup2",         // [dest, data, dest, src, remaining]
            "mstore",       // [dest, src, remaining]

            // Advance
            0x20,
            "add",          // dest+32
            "swap1",
            0x20,
            "add",          // src+32
            "swap1",
            "swap2",
            0x20,
            "swap1",
            "sub",
            "swap2",

            "copy_loop",
            "jump"
        ]],

        ["copy_done", [
            "pop",
            "pop",
            "pop",
            0x00,
            "mstore"
        ]]
    ]);

    // Should compile successfully
    assert!(bytecode.len() > 0);

    // Should contain 2 JUMPDESTs
    let jumpdest_count = bytecode.iter().filter(|&&b| b == 0x5b).count();
    assert_eq!(jumpdest_count, 2);
}
