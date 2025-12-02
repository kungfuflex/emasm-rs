use crate::*;

#[test]
fn test_simple_push_and_add() {
    let bytecode = evm_asm!([
        0x01,
        0x02,
        "add",
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]);
    
    // PUSH1 0x01, PUSH1 0x02, ADD, PUSH1 0x00, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
    assert_eq!(hex::encode(&bytecode), "600160020160005260206000f3");
}

#[test]
fn test_with_labels() {
    let bytecode = evm_asm!([
        0x01,
        "target",
        "jump",
        ["target", [
            0x02,
            "add",
            0x00,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]);
    

    // Should have PUSH for jump target, JUMP, then JUMPDEST at target
    assert!(bytecode.len() > 0);
    assert!(bytecode.contains(&0x5b)); // JUMPDEST opcode
    
    // Check that JUMP target matches JUMPDEST position
    // bytecode should be: PUSH1 0x01, PUSH1 XX (target), JUMP, JUMPDEST, ...
    //                     0     1    2     3         4     5
    // So JUMPDEST is at position 5, and XX should be 0x05
    assert_eq!(bytecode[3], 0x05, "Jump target should point to JUMPDEST at position 5");
}

#[test]
fn test_nested_segments() {
    let bytecode = evm_asm!([
        0x01,
        "start",
        "jump",
        ["start", [
            0x02,
            "add",
            "end",
            "jump"
        ]],
        ["end", [
            0x00,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]);
    
    // Should contain two JUMPDESTs
    let jumpdest_count = bytecode.iter().filter(|&&b| b == 0x5b).count();
    assert_eq!(jumpdest_count, 2);
}

#[test]
fn test_push_zero() {
    let bytecode = evm_asm!([0x00, "mstore"]);
    
    // Should use PUSH1 0x00 for compatibility
    assert_eq!(bytecode[0], 0x60); // PUSH1
    assert_eq!(bytecode[1], 0x00); // data
    assert_eq!(bytecode[2], 0x52); // MSTORE
}
