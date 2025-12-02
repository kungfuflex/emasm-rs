pub use emasm_common::{Assembler, AsmElement, AssemblerError, EVMEncodable};
pub use emasm_macros::{evm_asm, evm_asm_interpolator};

#[cfg(test)]
mod tests;
