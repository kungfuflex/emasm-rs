use crate::*;

#[test]
fn test_label_offset_calculation() {
    // Minimal case that shows the problem
    let bytecode = evm_asm!([
        0x00,            // PUSH1 0x00 - 2 bytes (offset 0-1)
        "mload",         // MLOAD - 1 byte (offset 2)
        "target",        // PUSH1 <offset> - should be 2 bytes if offset < 256
        "jump",          // JUMP - 1 byte
        
        ["target", [     // JUMPDEST should be here
            0x42,
            0x00,
            "mstore"
        ]]
    ]);
    
    eprintln!("Bytecode: {}", hex::encode(&bytecode));
    
    // Manually calculate what the offset should be:
    // Position 0-1: PUSH1 0x00
    // Position 2: MLOAD
    // Position 3-4: PUSH1 <target_offset>  <- This references "target"
    // Position 5: JUMP
    // Position 6: JUMPDEST <- This is where "target" points
    
    eprintln!("Expected: JUMPDEST at position 6");
    eprintln!("Actual JUMPDEST at position: {}", 
        bytecode.iter().position(|&b| b == 0x5b).unwrap());
    
    assert_eq!(bytecode[3], 0x60, "Should be PUSH1");
    let target_offset = bytecode[4];
    eprintln!("Target offset in bytecode: {}", target_offset);
    eprintln!("Byte at target offset: 0x{:02x}", bytecode[target_offset as usize]);
    
    assert_eq!(bytecode[target_offset as usize], 0x5b, 
        "Target should point to JUMPDEST");
}

#[test]
fn test_circular_reference_issue() {
    // The problem: when we reference a label BEFORE defining it,
    // and the label's position depends on the size of its own reference
    
    let bytecode = evm_asm!([
        // Assume we're at position 0
        "far_label",     // Reference to far_label
        "jump",
        
        // Add some padding to push the label further
        0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
        
        ["far_label", [  // This label is far enough that...
            0x42
        ]]
    ]);
    
    eprintln!("Bytecode with far label: {}", hex::encode(&bytecode));
    
    // Position 0-1: PUSH1 <offset> (if offset < 256)
    // OR Position 0-2: PUSH2 <offset> (if offset >= 256)
    
    let first_byte = bytecode[0];
    let push_size = if first_byte == 0x60 {
        eprintln!("Using PUSH1 (label < 256)");
        1
    } else if first_byte == 0x61 {
        eprintln!("Using PUSH2 (label >= 256)");
        2
    } else {
        panic!("Expected PUSH1 or PUSH2");
    };
    
    let jumpdest_pos = bytecode.iter().position(|&b| b == 0x5b).unwrap();
    eprintln!("JUMPDEST at position: {}", jumpdest_pos);
    eprintln!("Push instruction size: {}", push_size);
}

#[test]
fn test_self_referential_label() {
    // The root issue: a label that references itself creates a circular dependency
    // Position of label depends on size of reference, which depends on position
    
    // Example:
    // If label is at position 5 (< 256), we use PUSH1 (2 bytes total)
    // If label is at position 256 (>= 256), we use PUSH2 (3 bytes total)
    // But using PUSH2 instead of PUSH1 adds 1 byte, shifting the label!
    
    let bytecode = evm_asm!([
        "loop",
        "jump",
        
        ["loop", [
            "dup1",
            "iszero",
            "done",
            "jumpi",
            
            0x01,
            "sub",
            "loop",      // Self-reference!
            "jump"
        ]],
        
        ["done", [
            "pop"
        ]]
    ]);
    
    eprintln!("Self-referential bytecode: {}", hex::encode(&bytecode));
    
    // Find all label references (PUSH before JUMP/JUMPI)
    for (i, &byte) in bytecode.iter().enumerate() {
        if byte == 0x56 || byte == 0x57 {  // JUMP or JUMPI
            let opname = if byte == 0x56 { "JUMP" } else { "JUMPI" };
            
            // Look backwards for PUSH
            if i >= 2 && bytecode[i-2] >= 0x60 && bytecode[i-2] <= 0x7f {
                let push_bytes = bytecode[i-2] - 0x5f;
                let target = if push_bytes == 1 {
                    bytecode[i-1] as usize
                } else {
                    ((bytecode[i-1] as usize) << 8) | (bytecode[i] as usize)
                };
                
                eprintln!("{} at position {} -> target {}", opname, i, target);
                if target < bytecode.len() {
                    eprintln!("  Byte at target: 0x{:02x} {}", 
                        bytecode[target],
                        if bytecode[target] == 0x5b { "✓" } else { "✗" }
                    );
                }
            }
        }
    }
}
