use crate::*;
use alloy_primitives::Address;

#[test]
fn test_decode_actual_bytecode() {
    let test_address = Address::from([0x9f, 0xe4, 0x67, 0x36, 0x67, 0x9d, 0x2d, 0x9a, 0x65, 0xf0, 0x99, 0x2f, 0x22, 0x72, 0xde, 0x9f, 0x3c, 0x7f, 0xa6, 0xe0]);
    
    let builder = evm_asm_interpolator!([
        0x8b25f90e,      // selector
        0x00,
        "mstore",
        
        0x20,            // retSize
        0x00,            // retOffset
        0x04,            // argsSize
        0x1c,            // argsOffset
        &[0],            // address placeholder
        0xffff,          // gas
        "staticcall",
        "pop",
        
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
    let hex_str = hex::encode(&bytecode);
    
    println!("\nFull bytecode: {}", hex_str);
    println!("Length: {} bytes\n", bytecode.len());
    
    // Decode byte by byte
    let mut pos = 0;
    while pos < bytecode.len() {
        let opcode = bytecode[pos];
        print!("Position {:3} (0x{:02x}): ", pos, pos);
        
        match opcode {
            0x00 => println!("STOP"),
            0x01 => println!("ADD"),
            0x14 => println!("EQ"),
            0x15 => println!("ISZERO"),
            0x50 => println!("POP"),
            0x51 => println!("MLOAD"),
            0x52 => println!("MSTORE"),
            0x56 => println!("JUMP"),
            0x57 => println!("JUMPI"),
            0x5b => println!("JUMPDEST *** TARGET ***"),
            0x80 => println!("DUP1"),
            0xf3 => println!("RETURN"),
            0xfa => println!("STATICCALL"),
            0x60..=0x7f => {
                let push_bytes = (opcode - 0x5f) as usize;
                let mut data = String::new();
                for i in 1..=push_bytes {
                    if pos + i < bytecode.len() {
                        data.push_str(&format!("{:02x}", bytecode[pos + i]));
                    }
                }
                println!("PUSH{} 0x{} (decimal: {})", push_bytes, data, u128::from_str_radix(&data, 16).unwrap_or(0));
                pos += push_bytes;
            }
            _ => println!("UNKNOWN/OTHER (0x{:02x})", opcode),
        }
        pos += 1;
    }
    
    println!("\n=== JUMP ANALYSIS ===");
    // Find all JUMP/JUMPI and their targets
    pos = 0;
    while pos < bytecode.len() {
        if bytecode[pos] == 0x56 {  // JUMP
            // Look backwards for the PUSH
            if pos >= 2 && bytecode[pos-2] >= 0x60 && bytecode[pos-2] <= 0x7f {
                let target = bytecode[pos-1];
                println!("JUMP at position {} -> target 0x{:02x} ({})", pos, target, target);
                if (target as usize) < bytecode.len() {
                    println!("  Byte at target: 0x{:02x} {}", 
                        bytecode[target as usize],
                        if bytecode[target as usize] == 0x5b { "✓ JUMPDEST" } else { "✗ NOT JUMPDEST!" }
                    );
                }
            }
        }
        if bytecode[pos] == 0x57 {  // JUMPI
            // Look backwards for the PUSH (should be 2 positions back after condition)
            if pos >= 4 && bytecode[pos-4] >= 0x60 && bytecode[pos-4] <= 0x7f {
                let target = bytecode[pos-3];
                println!("JUMPI at position {} -> target 0x{:02x} ({})", pos, target, target);
                if (target as usize) < bytecode.len() {
                    println!("  Byte at target: 0x{:02x} {}", 
                        bytecode[target as usize],
                        if bytecode[target as usize] == 0x5b { "✓ JUMPDEST" } else { "✗ NOT JUMPDEST!" }
                    );
                }
            }
        }
        pos += 1;
    }
}
