use crate::*;

#[test]
fn test_simple_jump_to_label() {
    // Simplest possible case: jump to a label
    let bytecode = evm_asm!([
        "target",
        "jump",
        ["target", [
            0x42,
            0x00,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]);
    
    eprintln!("Bytecode: {}", hex::encode(&bytecode));
    
    // Decode to verify structure
    // Should be: PUSH1 <target_addr>, JUMP, JUMPDEST, PUSH1 0x42, ...
    assert_eq!(bytecode[0], 0x60); // PUSH1
    let target_addr = bytecode[1];
    assert_eq!(bytecode[2], 0x56); // JUMP
    assert_eq!(bytecode[target_addr as usize], 0x5b); // JUMPDEST at target
}

#[test]
fn test_conditional_jump_pattern() {
    // Pattern similar to what we're generating: check condition, jump if true
    let bytecode = evm_asm!([
        0x01,
        "iszero",        // Check if zero
        "finish",        // Push finish address
        "jumpi",         // Conditional jump
        0x42,            // This should execute if not zero
        ["finish", [
            0x00,
            "return"
        ]]
    ]);
    
    eprintln!("Conditional jump bytecode: {}", hex::encode(&bytecode));
    
    // Verify JUMPDEST is at the right location
    let finish_label_push_idx = 3; // After PUSH1 0x01 (pos 0-1), ISZERO (pos 2), PUSH1 for label (pos 3)
    let finish_addr = bytecode[finish_label_push_idx + 1]; // Get the address from PUSH1 (pos 4)
    
    eprintln!("finish_addr from bytecode: 0x{:02x} (byte {})", finish_addr, finish_addr);
    eprintln!("Bytecode at that position: 0x{:02x}", bytecode[finish_addr as usize]);
    
    assert_eq!(bytecode[finish_addr as usize], 0x5b, "JUMPDEST should be at the address pushed");
}

#[test]
fn test_loop_with_jumpi() {
    // Minimal loop pattern: decrement and loop back if not zero
    let bytecode = evm_asm!([
        0x03,            // Counter
        "loop_start",
        "jump",
        
        ["loop_start", [
            "dup1",          // Duplicate counter
            "iszero",        // Check if zero
            "finish",        // Push finish address
            "jumpi",         // Jump to finish if zero
            
            0x01,
            "sub",           // Decrement counter
            "loop_start",    // Push loop_start address
            "jump"           // Jump back
        ]],
        
        ["finish", [
            "pop",           // Pop counter
            0x00,
            0x00,
            "return"
        ]]
    ]);
    
    eprintln!("Loop bytecode: {}", hex::encode(&bytecode));
    
    // Find where loop_start JUMPDEST is
    let mut jumpdest_positions = Vec::new();
    for (i, &byte) in bytecode.iter().enumerate() {
        if byte == 0x5b {
            jumpdest_positions.push(i);
            eprintln!("Found JUMPDEST at position {}", i);
        }
    }
    
    assert_eq!(jumpdest_positions.len(), 2, "Should have exactly 2 JUMPDESTs (loop_start and finish)");
}

#[test]
fn test_emasm_rs_issue_minimal() {
    // Recreate the exact pattern from alkanes-rs that's failing
    let bytecode = evm_asm!([
        0x05,            // Some value (simulating payments_length)
        "loop_start",    // Push loop_start address
        "jump",          // Jump to loop_start
        
        ["loop_start", [
            "dup1",          // [idx, idx]
            "iszero",        // [idx==0, idx]
            "finish",        // [finish_addr, idx==0, idx]
            "jumpi",         // Jump to finish if idx==0
            
            0x01,
            "sub",           // [idx-1]
            
            "loop_start",    // Push loop_start address
            "jump"           // Jump back
        ]],
        
        ["finish", [
            "pop",           // Pop idx=0
            0x00,            // offset
            0x00,            // size
            "return"
        ]]
    ]);
    
    eprintln!("Alkanes pattern bytecode: {}", hex::encode(&bytecode));
    
    // Manually verify label resolution
    // After PUSH1 0x05, we should have PUSH1 <loop_addr>, JUMP
    assert_eq!(bytecode[0], 0x60, "Should be PUSH1");
    assert_eq!(bytecode[1], 0x05, "Should be value 5");
    assert_eq!(bytecode[2], 0x60, "Should be PUSH1 for loop_start");
    let loop_addr = bytecode[3];
    assert_eq!(bytecode[4], 0x56, "Should be JUMP");
    
    eprintln!("Loop address from bytecode: 0x{:02x} (byte {})", loop_addr, loop_addr);
    eprintln!("Byte at loop_addr: 0x{:02x}", bytecode[loop_addr as usize]);
    
    assert_eq!(
        bytecode[loop_addr as usize], 
        0x5b, 
        "JUMPDEST should be at position {} but found opcode 0x{:02x}", 
        loop_addr,
        bytecode[loop_addr as usize]
    );
}

#[test]
fn test_staticcall_then_jump_pattern() {
    // Even more minimal: just do a staticcall then jump
    use alloy_primitives::Address;
    
    let test_address = Address::from([0x9f; 20]);
    
    let builder = evm_asm_interpolator!([
        // Store selector at memory 0x00
        0x8b25f90e,      // selector
        0x00,
        "mstore",
        
        // STATICCALL
        0x20,            // retSize
        0x00,            // retOffset
        0x04,            // argsSize
        0x1c,            // argsOffset
        &[0],            // address placeholder
        0xffff,          // gas
        "staticcall",
        "pop",           // Pop success
        
        // Load result and jump
        0x00,
        "mload",
        "loop_start",
        "jump",
        
        ["loop_start", [
            "dup1",
            "iszero",
            "finish",
            "jumpi",
            
            0x01,
            "sub",
            "loop_start",
            "jump"
        ]],
        
        ["finish", [
            "pop",
            0x00,
            0x00,
            "return"
        ]]
    ]);
    
    let bytecode = builder(Box::new(test_address));
    
    eprintln!("Full pattern bytecode: {}", hex::encode(&bytecode));
    eprintln!("Bytecode length: {} bytes", bytecode.len());
    
    // Find all JUMPDESTs
    for (i, &byte) in bytecode.iter().enumerate() {
        if byte == 0x5b {
            eprintln!("JUMPDEST at position {}", i);
        }
        if byte == 0x56 {
            eprintln!("JUMP at position {} (should have pushed target just before)", i);
        }
        if byte == 0x57 {
            eprintln!("JUMPI at position {} (should have pushed target and condition before)", i);
        }
    }
}
