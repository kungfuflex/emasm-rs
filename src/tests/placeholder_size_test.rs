use crate::*;
use alloy_primitives::Address;

#[test]
fn test_placeholder_size_calculation() {
    let test_address = Address::from([0xAA; 20]);
    
    // Build without interpolation first - use the address directly as a literal
    let bytecode_direct = evm_asm!([
        0x8b25f90e,
        0x00,
        "mstore",
        "target",
        "jump",
        ["target", [
            0x42
        ]]
    ]);
    
    eprintln!("Direct bytecode (no address): {}", hex::encode(&bytecode_direct));
    eprintln!("Length: {}", bytecode_direct.len());
    
    // Now with interpolator
    let builder = evm_asm_interpolator!([
        0x8b25f90e,
        0x00,
        "mstore",
        &[0],            // Placeholder for address
        "target",
        "jump",
        ["target", [
            0x42
        ]]
    ]);
    
    let bytecode_with_addr = builder(Box::new(test_address));
    
    eprintln!("With address bytecode: {}", hex::encode(&bytecode_with_addr));
    eprintln!("Length: {}", bytecode_with_addr.len());
    eprintln!("Difference: {} bytes", bytecode_with_addr.len() as i32 - bytecode_direct.len() as i32);
    
    // Find JUMPDEST
    let jumpdest_pos_direct = bytecode_direct.iter().position(|&b| b == 0x5b).unwrap();
    let jumpdest_pos_with_addr = bytecode_with_addr.iter().position(|&b| b == 0x5b).unwrap();
    
    eprintln!("JUMPDEST without address: position {}", jumpdest_pos_direct);
    eprintln!("JUMPDEST with address: position {}", jumpdest_pos_with_addr);
    eprintln!("Shift: {} bytes", jumpdest_pos_with_addr as i32 - jumpdest_pos_direct as i32);
    
    // Check if label points to correct position
    // Find the PUSH before JUMP
    let jump_pos = bytecode_with_addr.iter().position(|&b| b == 0x56).unwrap();
    eprintln!("JUMP at position {}", jump_pos);
    
    if jump_pos >= 2 {
        let push_opcode = bytecode_with_addr[jump_pos - 2];
        if push_opcode >= 0x60 && push_opcode <= 0x7f {
            let target = bytecode_with_addr[jump_pos - 1] as usize;
            eprintln!("Target from PUSH: {}", target);
            eprintln!("Expected target (JUMPDEST position): {}", jumpdest_pos_with_addr);
            eprintln!("Difference: {} bytes", target as i32 - jumpdest_pos_with_addr as i32);
            
            assert_eq!(target, jumpdest_pos_with_addr, "Label should point to JUMPDEST!");
        }
    }
}

#[test]
fn test_address_encoding_size() {
    use alloy_primitives::Address;
    use crate::EVMEncodable;
    
    let addr = Address::from([0x9f, 0xe4, 0x67, 0x36, 0x67, 0x9d, 0x2d, 0x9a, 
                               0x65, 0xf0, 0x99, 0x2f, 0x22, 0x72, 0xde, 0x9f, 
                               0x3c, 0x7f, 0xa6, 0xe0]);
    
    let bytes = addr.to_evm_bytes();
    eprintln!("Address as EVM bytes: {} bytes", bytes.len());
    eprintln!("Hex: {}", hex::encode(&bytes));
    
    // When this becomes a Literal in ASM, it will be encoded as PUSH20
    // PUSH20 = 1 byte opcode + 20 bytes data = 21 bytes total
    eprintln!("As PUSH instruction: {} bytes total (1 opcode + {} data)", 1 + bytes.len(), bytes.len());
}
