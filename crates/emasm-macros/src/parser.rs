use syn::{Expr, ExprLit, ExprReference, Lit, punctuated::Punctuated, Token};

#[derive(Debug, Clone)]
pub enum AsmToken {
    Opcode(String),
    Literal(u128),
    HexLiteral(Vec<u8>),
    Label(String),
    Segment(String, Vec<AsmToken>),
    BytesSegment(String, Vec<u8>),
    BytesPtr(String),
    BytesSize(String),
    Placeholder(usize),
}

pub fn parse_asm_elements(
    exprs: &Punctuated<Expr, Token![,]>,
) -> Result<Vec<AsmToken>, String> {
    let mut result = Vec::new();
    
    for expr in exprs {
        result.push(parse_single_element(expr)?);
    }
    
    Ok(result)
}

fn parse_single_element(expr: &Expr) -> Result<AsmToken, String> {
    match expr {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => {
            let value = s.value();
            
            if value.starts_with("bytes:") {
                if value.ends_with(":ptr") {
                    let label = value.strip_prefix("bytes:")
                        .and_then(|s| s.strip_suffix(":ptr"))
                        .unwrap()
                        .to_string();
                    return Ok(AsmToken::BytesPtr(label));
                } else if value.ends_with(":size") {
                    let label = value.strip_prefix("bytes:")
                        .and_then(|s| s.strip_suffix(":size"))
                        .unwrap()
                        .to_string();
                    return Ok(AsmToken::BytesSize(label));
                }
            }
            
            Ok(AsmToken::Opcode(value))
        }
        
        Expr::Lit(ExprLit { lit: Lit::Int(i), .. }) => {
            // Get the raw token string to handle hex literals > u128
            let token_str = i.to_string();

            if token_str.starts_with("0x") || token_str.starts_with("0X") {
                // Try parsing as u128 first
                match i.base10_parse::<u128>() {
                    Ok(value) => Ok(AsmToken::Literal(value)),
                    Err(_) => {
                        // Parse as hex bytes for values > u128 (up to 256-bit for EVM)
                        let hex_str = token_str.strip_prefix("0x")
                            .or_else(|| token_str.strip_prefix("0X"))
                            .unwrap();
                        let hex_bytes = parse_hex_string(hex_str)?;
                        Ok(AsmToken::HexLiteral(hex_bytes))
                    }
                }
            } else {
                let value = i.base10_parse::<u128>()
                    .map_err(|e| format!("Failed to parse integer: {}", e))?;
                Ok(AsmToken::Literal(value))
            }
        }
        
        Expr::Array(arr) => {
            if arr.elems.len() < 2 {
                return Err("Segment array must have at least 2 elements".to_string());
            }
            
            let first = &arr.elems[0];
            
            if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = first {
                let label = s.value();
                
                if label.starts_with("bytes:") {
                    let second = &arr.elems[1];
                    if let Expr::Lit(ExprLit { lit: Lit::Str(hex_str), .. }) = second {
                        let hex_data = parse_hex_string(&hex_str.value())?;
                        return Ok(AsmToken::BytesSegment(label, hex_data));
                    }
                    return Err("Bytes segment must have hex string as second element".to_string());
                }
                
                let second = &arr.elems[1];
                if let Expr::Array(inner_arr) = second {
                    let inner_elements = parse_asm_elements(&inner_arr.elems)?;
                    return Ok(AsmToken::Segment(label, inner_elements));
                }
                
                return Err("Segment must have array as second element".to_string());
            }
            
            Err("Segment array must start with string label".to_string())
        }
        
        Expr::Reference(ExprReference { expr: inner, .. }) => {
            if let Expr::Array(arr) = &**inner {
                if arr.elems.len() == 1 {
                    if let Expr::Lit(ExprLit { lit: Lit::Int(i), .. }) = &arr.elems[0] {
                        let idx = i.base10_parse::<usize>()
                            .map_err(|e| format!("Failed to parse placeholder index: {}", e))?;
                        return Ok(AsmToken::Placeholder(idx));
                    }
                }
            }
            Err("Invalid placeholder syntax, expected &[index]".to_string())
        }
        
        _ => Err(format!("Unsupported expression type in assembly")),
    }
}

fn parse_hex_string(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    
    let s = if s.len() % 2 != 0 {
        format!("0{}", s)
    } else {
        s.to_string()
    };
    
    hex::decode(&s).map_err(|e| format!("Invalid hex string: {}", e))
}
