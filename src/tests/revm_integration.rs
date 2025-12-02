use crate::*;
use revm::{
    primitives::{Address, Bytecode, Bytes, TxKind, U256},
    Evm,
    InMemoryDB,
};

fn execute_bytecode(code: Vec<u8>) -> Result<Bytes, String> {
    use revm::primitives::AccountInfo;
    
    let mut db = InMemoryDB::default();
    
    let contract_address = Address::from([0x42; 20]);
    let caller_address = Address::from([0x41; 20]);
    
    // Insert contract code into database
    let bytecode = Bytecode::new_raw(Bytes::from(code));
    let account_info = AccountInfo {
        balance: U256::ZERO,
        nonce: 1,
        code_hash: bytecode.hash_slow(),
        code: Some(bytecode),
    };
    db.insert_account_info(contract_address, account_info);
    
    let mut evm = Evm::builder()
        .with_db(db)
        .modify_tx_env(|tx| {
            tx.caller = caller_address;
            tx.transact_to = TxKind::Call(contract_address);
            tx.data = Bytes::new();
            tx.value = U256::ZERO;
        })
        .build();
    
    let result = evm.transact()
        .map_err(|e| format!("Transaction failed: {:?}", e))?;
    
    match result.result {
        revm::primitives::ExecutionResult::Success { output, .. } => {
            match output {
                revm::primitives::Output::Call(data) => Ok(data),
                revm::primitives::Output::Create(data, _) => Ok(data),
            }
        }
        revm::primitives::ExecutionResult::Revert { output, .. } => {
            Err(format!("Reverted: {}", hex::encode(output)))
        }
        revm::primitives::ExecutionResult::Halt { reason, .. } => {
            Err(format!("Halted: {:?}", reason))
        }
    }
}

#[test]
fn test_return_constant() {
    // Returns the value 0x42 in memory
    let bytecode = evm_asm!([
        0x42,
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]);
    
    let result = execute_bytecode(bytecode).expect("Execution failed");
    
    // Should return 32 bytes with 0x42 as the last byte
    assert_eq!(result.len(), 32);
    assert_eq!(result[31], 0x42);
}

#[test]
fn test_addition() {
    // Computes 10 + 32 and returns it
    let bytecode = evm_asm!([
        0x0a,
        0x20,
        "add",
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]);
    
    let result = execute_bytecode(bytecode).expect("Execution failed");
    
    assert_eq!(result.len(), 32);
    assert_eq!(result[31], 0x2a); // 10 + 32 = 42
}

#[test]
fn test_interpolated_execution() {
    let builder = evm_asm_interpolator!([
        &[0],
        &[1],
        "add",
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]);
    
    let bytecode = builder(Box::new(15u8), Box::new(27u8));
    let result = execute_bytecode(bytecode).expect("Execution failed");
    
    assert_eq!(result.len(), 32);
    assert_eq!(result[31], 42); // 15 + 27 = 42
}

#[test]
fn test_conditional_jump() {
    // If 1 == 1, jump to success, else halt
    let bytecode = evm_asm!([
        0x01,
        0x01,
        "eq",
        "success",
        "jumpi",
        "invalid",
        ["success", [
            0x42,
            0x00,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]);
    

    let result = execute_bytecode(bytecode).expect("Execution failed");
    
    assert_eq!(result.len(), 32);
    assert_eq!(result[31], 0x42);
}
