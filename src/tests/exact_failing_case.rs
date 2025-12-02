use crate::*;
use alloy_primitives::Address;

#[test]
fn test_exact_alkanes_structure() {
    // This is the EXACT structure from alkanes-rs that's failing
    let test_address = Address::from([0x9f, 0xe4, 0x67, 0x36, 0x67, 0x9d, 0x2d, 0x9a, 
                                       0x65, 0xf0, 0x99, 0x2f, 0x22, 0x72, 0xde, 0x9f, 
                                       0x3c, 0x7f, 0xa6, 0xe0]);
    
    let builder = evm_asm_interpolator!([
        0x8b25f90e,      // PUSH4 selector
        0x00,            // PUSH1 0x00
        "mstore",        // MSTORE
        
        0x20,            // PUSH1 0x20
        0x00,            // PUSH1 0x00
        0x04,            // PUSH1 0x04
        0x1c,            // PUSH1 0x1c
        &[0],            // PUSH20 address (placeholder)
        0xffff,          // PUSH2 0xffff
        "staticcall",    // STATICCALL
        "pop",           // POP
        
        0x00,            // PUSH1 0x00
        "mload",         // MLOAD
        "loop_start",    // PUSH1 <loop_start>  ← This should reference position of JUMPDEST
        "jump",          // JUMP
        
        ["loop_start", [ // JUMPDEST should be here
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
    let hex_str = hex::encode(&bytecode);
    
    eprintln!("\n=== EXACT ALKANES CASE ===");
    eprintln!("Bytecode: {}", hex_str);
    eprintln!("Length: {} bytes\n", bytecode.len());
    
    // Decode to find the issue
    let mut pos = 0;
    eprintln!("Key positions:");
    while pos < bytecode.len() {
        if bytecode[pos] == 0x56 {  // JUMP
            eprintln!("  JUMP at position {}", pos);
            if pos >= 2 {
                let target = bytecode[pos - 1];
                eprintln!("    Target: {} (0x{:02x})", target, target);
                if (target as usize) < bytecode.len() {
                    eprintln!("    Byte at target: 0x{:02x} {}", 
                        bytecode[target as usize],
                        if bytecode[target as usize] == 0x5b { "✓ JUMPDEST" } else { "✗ NOT JUMPDEST!" }
                    );
                }
            }
        }
        if bytecode[pos] == 0x5b {  // JUMPDEST
            eprintln!("  JUMPDEST at position {}", pos);
        }
        pos += 1;
    }
    
    // The test should pass - let's verify
    let first_jump_pos = bytecode.iter().position(|&b| b == 0x56).unwrap();
    let first_jumpdest_pos = bytecode.iter().position(|&b| b == 0x5b).unwrap();
    
    eprintln!("\nFirst JUMP at: {}", first_jump_pos);
    eprintln!("First JUMPDEST at: {}", first_jumpdest_pos);
    
    let target = bytecode[first_jump_pos - 1] as usize;
    eprintln!("First JUMP targets: {}", target);
    
    assert_eq!(target, first_jumpdest_pos, 
        "First JUMP should target first JUMPDEST! Expected {} but got {}", 
        first_jumpdest_pos, target);
}
