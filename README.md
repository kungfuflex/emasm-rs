# emasm-rs

A Rust-based EVM assembler with powerful macro support for writing EVM bytecode programs with automatic label resolution and runtime value interpolation.

Inspired by the [emasm](https://github.com/kungfuflex/emasm) JavaScript library, **emasm-rs** brings type-safe, compile-time EVM assembly to Rust with zero-cost abstractions and a declarative macro syntax.

## Features

- 🚀 **Declarative Syntax**: Write EVM assembly using Rust macro syntax with nested arrays
- 🎯 **Automatic Label Resolution**: Define jump labels and the assembler automatically calculates offsets
- ⚡ **Efficient Bytecode**: Minimal-width PUSH instruction encoding for optimal bytecode size
- 🔄 **Runtime Interpolation**: Create parameterized assembly functions with `evm_asm_interpolator!`
- 🛡️ **Type-Safe**: Leverage Rust's type system with the `EVMEncodable` trait
- 📦 **Monorepo Structure**: Organized into focused crates for common functionality, macros, and CLIs
- ✅ **Well-Tested**: Comprehensive test suite with revm integration

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage Guide](#usage-guide)
  - [Basic Assembly](#basic-assembly)
  - [Labels and Control Flow](#labels-and-control-flow)
  - [Runtime Interpolation](#runtime-interpolation)
  - [Bytes Segments](#bytes-segments)
  - [Nested Segments](#nested-segments)
- [API Reference](#api-reference)
- [Architecture](#architecture)
- [Testing](#testing)
- [Examples](#examples)
- [License](#license)

## Installation

Add `emasm` to your `Cargo.toml`:

```toml
[dependencies]
emasm = { path = "path/to/emasm-rs" }
# For using alloy types
alloy-primitives = "0.8"
hex = "0.4"  # For encoding/decoding hex strings
```

## Quick Start

```rust
use emasm::evm_asm;

fn main() {
    // Simple program: compute 1 + 2 and return result
    let bytecode = evm_asm!([
        0x01,      // PUSH1 0x01
        0x02,      // PUSH1 0x02
        "add",     // ADD
        0x00,      // PUSH1 0x00
        "mstore",  // MSTORE
        0x20,      // PUSH1 0x20
        0x00,      // PUSH1 0x00
        "return"   // RETURN
    ]);
    
    println!("Bytecode: 0x{}", hex::encode(&bytecode));
    // Output: 0x600160020160005260206000f3
}
```

## Usage Guide

### Basic Assembly

The `evm_asm!` macro compiles EVM assembly at compile-time. It accepts an array of:
- **Integers** (0x00-0xFFFFFFFF...): Automatically encoded as minimal PUSH instructions
- **Strings**: EVM opcodes in lowercase (e.g., "add", "mstore", "jump")

```rust
use emasm::evm_asm;

// Store 0x42 at memory position 0 and return 32 bytes
let bytecode = evm_asm!([
    0x42,      // Value to store
    0x00,      // Memory position
    "mstore",  // Store to memory
    0x20,      // Return 32 bytes
    0x00,      // From position 0
    "return"
]);
```

**Supported Opcodes**: All EVM opcodes are supported including:
- Arithmetic: `add`, `mul`, `sub`, `div`, `mod`, `exp`, etc.
- Comparison: `lt`, `gt`, `eq`, `iszero`, etc.
- Bitwise: `and`, `or`, `xor`, `not`, `shl`, `shr`, `sar`
- Memory/Storage: `mload`, `mstore`, `sload`, `sstore`
- Control Flow: `jump`, `jumpi`, `jumpdest`, `stop`, `return`, `revert`
- Stack: `pop`, `dup1`-`dup16`, `swap1`-`swap16`
- And many more...

### Labels and Control Flow

Labels allow you to define jump destinations without manually calculating byte offsets.

**Syntax**:
- **Define a label**: `["label_name", [... code ...]]`
- **Reference a label**: `"label_name"` (automatically resolved to PUSH with offset)

```rust
use emasm::evm_asm;

// Conditional jump: if 1 == 1, jump to success branch
let bytecode = evm_asm!([
    0x01,
    0x01,
    "eq",           // Stack: [1] (true)
    "success",      // Label reference - pushes jump destination
    "jumpi",        // Conditional jump
    "invalid",      // Only executed if condition is false
    
    ["success", [   // Label definition - JUMPDEST inserted here
        0x42,
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]]
]);
```

**How it works**:
1. The assembler scans for all label definitions (`["name", [...]]`)
2. Label references (`"name"`) are replaced with PUSH instructions containing the offset
3. Label definitions are replaced with JUMPDEST followed by their code
4. Offsets are iteratively optimized for minimal PUSH width

**Nested Labels**:
```rust
let bytecode = evm_asm!([
    "outer",
    "jump",
    
    ["outer", [
        0x01,
        "inner",
        "jumpi",
        "revert",
        
        ["inner", [
            0x42,
            0x00,
            "mstore",
            0x20,
            0x00,
            "return"
        ]]
    ]]
]);
```

### Runtime Interpolation

The `evm_asm_interpolator!` macro creates a function that accepts runtime values and assembles them into bytecode.

**Placeholder Syntax**: `&[index]` where index starts at 0

```rust
use emasm::{evm_asm_interpolator, EVMEncodable};

// Create a parameterized assembly template
let add_and_return = evm_asm_interpolator!([
    &[0],           // First argument placeholder
    &[1],           // Second argument placeholder
    "add",          // Add them
    0x00,
    "mstore",
    0x20,
    0x00,
    "return"
]);

// Generate bytecode with specific values
let bytecode1 = add_and_return(Box::new(10u128), Box::new(20u128));
let bytecode2 = add_and_return(Box::new(100u128), Box::new(200u128));

// Each call produces different bytecode with interpolated values
```

**Placeholders in Segments**:
```rust
let conditional_return = evm_asm_interpolator!([
    &[0],              // Condition value
    "success",
    "jumpi",
    &[1],              // Failure value
    0x00,
    "mstore",
    0x20,
    0x00,
    "return",
    
    ["success", [
        &[2],          // Success value
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]]
]);

let bytecode = conditional_return(
    Box::new(1u8),      // condition = true
    Box::new(0xFFu8),   // failure value
    Box::new(0x42u8)    // success value
);
```

### Bytes Segments

For embedding raw data (useful for CODECOPY operations), use bytes segments.

**Syntax**:
- **Define bytes**: `["bytes:name", ["0xHEXDATA"]]`
- **Reference pointer**: `"bytes:name:ptr"`
- **Reference size**: `"bytes:name:size"`

```rust
use emasm::evm_asm;

let bytecode = evm_asm!([
    // Define a bytes segment
    ["bytes:data", ["0xdeadbeefcafebabe"]],
    
    // Copy bytes to memory
    "bytes:data:size",  // Size in bytes
    "bytes:data:ptr",   // Offset in bytecode
    0x00,               // Destination in memory
    "codecopy",
    
    // Return the copied data
    "bytes:data:size",
    0x00,
    "return"
]);
```

### Nested Segments

Segments can be nested arbitrarily deep:

```rust
let bytecode = evm_asm!([
    "main",
    "jump",
    
    ["main", [
        0x01,
        "branch_a",
        "jumpi",
        "branch_b",
        "jump",
        
        ["branch_a", [
            0x0a,
            0x00,
            "mstore",
            "done",
            "jump"
        ]],
        
        ["branch_b", [
            0x0b,
            0x00,
            "mstore",
            "done",
            "jump"
        ]],
        
        ["done", [
            0x20,
            0x00,
            "return"
        ]]
    ]]
]);
```

## API Reference

### Macros

#### `evm_asm!`

Compiles EVM assembly at compile-time.

```rust
let bytecode: Vec<u8> = evm_asm!([/* assembly */]);
```

**Returns**: `Vec<u8>` containing the assembled bytecode.

**Panics**: At compile-time if:
- Unknown opcode is used
- Invalid syntax (malformed labels, etc.)

#### `evm_asm_interpolator!`

Creates a closure that generates bytecode with runtime values.

```rust
let builder = evm_asm_interpolator!([/* assembly with &[N] placeholders */]);
let bytecode = builder(Box::new(value1), Box::new(value2), ...);
```

**Returns**: A closure with signature:
```rust
impl Fn(Box<dyn EVMEncodable>, ...) -> Vec<u8>
```

**Number of parameters**: Determined by highest placeholder index + 1.

### Traits

#### `EVMEncodable`

Trait for types that can be encoded as EVM values.

```rust
pub trait EVMEncodable {
    fn to_evm_bytes(&self) -> Vec<u8>;
}
```

**Implemented for**:
- Primitive integers: `u8`, `u16`, `u32`, `u64`, `u128`
- Alloy types: `U256`, `Address`, `FixedBytes<N>`, `Bytes`
- Raw bytes: `Vec<u8>`, `&[u8]`

**Encoding behavior**:
- Integers are encoded as big-endian bytes with leading zeros trimmed
- Zero is encoded as a single `0x00` byte
- Addresses and fixed bytes use their raw representation

**Custom implementations**:
```rust
struct MyType(u64);

impl EVMEncodable for MyType {
    fn to_evm_bytes(&self) -> Vec<u8> {
        self.0.to_evm_bytes()  // Delegate to u64
    }
}
```

### Types

#### `Assembler`

Low-level assembler for programmatic use.

```rust
use emasm::{Assembler, AsmElement};

let assembler = Assembler::new();
let elements = vec![
    AsmElement::Literal(vec![0x01]),
    AsmElement::Opcode("add".to_string()),
];
let bytecode = assembler.assemble(&elements)?;
```

## Architecture

### Monorepo Structure

```
emasm-rs/
├── Cargo.toml              # Workspace root
├── src/
│   ├── lib.rs              # Re-exports macros and traits
│   └── tests/              # Integration tests
│       ├── basic_assembly.rs
│       ├── interpolation.rs
│       └── revm_integration.rs
├── crates/
│   ├── emasm-common/       # Core assembler logic
│   │   ├── src/
│   │   │   ├── assembler.rs    # Main assembler
│   │   │   ├── opcodes.rs      # Opcode definitions
│   │   │   ├── types.rs        # AST types
│   │   │   └── encodable.rs    # EVMEncodable trait
│   │   └── Cargo.toml
│   ├── emasm-macros/       # Procedural macros
│   │   ├── src/
│   │   │   ├── lib.rs          # Macro definitions
│   │   │   └── parser.rs       # Macro input parser
│   │   └── Cargo.toml
│   ├── emasm-cli/          # CLI assembler
│   └── edisasm-cli/        # CLI disassembler
└── reference/              # Reference implementations
    ├── emasm/              # Original JS implementation
    └── huff-rs/            # Reference Rust project
```

### Assembly Process

1. **Parsing**: Macro input is parsed into `AsmElement` enum
2. **Label Collection**: All label definitions are identified
3. **First Pass**: Initial offset calculation with size estimates
4. **Optimization**: Iterative refinement until offsets stabilize
5. **Encoding**: Final bytecode generation with minimal PUSH widths

### Key Design Decisions

- **Zero-copy where possible**: References used throughout assembly
- **Iterative optimization**: Label offsets converge to minimal representation
- **Compile-time for static**: `evm_asm!` has no runtime overhead
- **Runtime for dynamic**: `evm_asm_interpolator!` allows parameterization

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test suite
cargo test --lib tests::basic_assembly
cargo test --lib tests::interpolation
cargo test --lib tests::revm_integration

# Build release
cargo build --release
```

### Test Coverage

- **Basic Assembly** (4 tests): Simple operations, literals, stack manipulation
- **Interpolation** (4 tests): Placeholders, multiple arguments, nested segments
- **REVM Integration** (4 tests): Full EVM execution validation

All tests execute actual bytecode using [revm](https://github.com/bluealloy/revm) to verify correctness.

## Examples

### Example 1: Simple Calculator

```rust
use emasm::evm_asm_interpolator;

// Create a calculator that supports add/sub operations
let calculator = evm_asm_interpolator!([
    &[0],              // operand1
    &[1],              // operand2
    &[2],              // operation (0=add, 1=sub)
    "is_sub",
    "jumpi",
    
    // Add branch
    "add",
    "result",
    "jump",
    
    // Sub branch
    ["is_sub", [
        "sub",
        "result",
        "jump"
    ]],
    
    // Return result
    ["result", [
        0x00,
        "mstore",
        0x20,
        0x00,
        "return"
    ]]
]);

let add_result = calculator(Box::new(10u8), Box::new(5u8), Box::new(0u8));
let sub_result = calculator(Box::new(10u8), Box::new(5u8), Box::new(1u8));
```

### Example 2: Memory Operations

```rust
use emasm::evm_asm;

// Copy calldata to memory and return it
let bytecode = evm_asm!([
    "calldatasize",  // Get size of calldata
    0x00,            // Source offset in calldata
    0x00,            // Destination in memory
    "calldatacopy",  // Copy calldata to memory
    
    "calldatasize",  // Size to return
    0x00,            // Offset in memory
    "return"
]);
```

### Example 3: Storage Contract

```rust
use emasm::evm_asm_interpolator;

// Simple storage contract: stores value at key
let storage_contract = evm_asm_interpolator!([
    // Check if we're reading or writing
    "calldatasize",
    0x20,
    "eq",
    "write",
    "jumpi",
    
    // Read: Load from storage slot 0
    0x00,
    "sload",
    0x00,
    "mstore",
    0x20,
    0x00,
    "return",
    
    // Write: Store to slot 0
    ["write", [
        0x00,
        "calldataload",  // Load value from calldata
        0x00,
        "sstore",        // Store to slot 0
        0x00,
        0x00,
        "return"
    ]]
]);

let bytecode = storage_contract();
```

### Example 4: Using Alloy Types

```rust
use emasm::{evm_asm_interpolator, EVMEncodable};
use alloy_primitives::{U256, Address};

let transfer = evm_asm_interpolator!([
    // Push recipient address (160 bits)
    &[0],
    
    // Push amount (256 bits)
    &[1],
    
    // ... rest of transfer logic
]);

let recipient = Address::from([0x42; 20]);
let amount = U256::from(1000000000000000000u128); // 1 ETH

let bytecode = transfer(Box::new(recipient), Box::new(amount));
```

## Comparison with Reference Implementation

This project is inspired by [emasm](./reference/emasm), a JavaScript EVM assembler. Key improvements:

| Feature | emasm (JS) | emasm-rs (Rust) |
|---------|-----------|-----------------|
| Type Safety | Runtime | Compile-time |
| Performance | JIT optimized | Zero-cost abstractions |
| Interpolation | String templates | Type-safe closures |
| Error Detection | Runtime | Compile-time |
| IDE Support | Limited | Full LSP support |
| Memory Safety | GC | Rust ownership |

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Areas for improvement:
- Implement CLI tools (`emasm-cli`, `edisasm-cli`)
- Add more EVM opcode validations
- Optimize assembly algorithm further
- Add macro for inline assembly in Rust contracts
- Support for EVM versions and fork-specific opcodes

## Credits

- Original concept inspired by [emasm](./reference/emasm) by flex
- Reference architecture from [huff-rs](./reference/huff-rs)
- EVM execution testing via [revm](https://github.com/bluealloy/revm)
