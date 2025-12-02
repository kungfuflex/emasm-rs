use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ExprArray};

mod parser;
use parser::{parse_asm_elements, AsmToken};

#[proc_macro]
pub fn evm_asm(input: TokenStream) -> TokenStream {
    let input_array = parse_macro_input!(input as ExprArray);
    
    match parse_asm_elements(&input_array.elems) {
        Ok(elements) => {
            // Collect all defined labels
            let mut defined_labels = std::collections::HashSet::new();
            fn collect_labels(elem: &AsmToken, labels: &mut std::collections::HashSet<String>) {
                match elem {
                    AsmToken::Segment(name, inner) => {
                        labels.insert(name.clone());
                        for e in inner {
                            collect_labels(e, labels);
                        }
                    }
                    AsmToken::BytesSegment(name, _) => {
                        labels.insert(name.clone());
                    }
                    _ => {}
                }
            }
            for elem in &elements {
                collect_labels(elem, &mut defined_labels);
            }
            
            let element_tokens = elements.into_iter().map(|elem| match elem {
                AsmToken::Placeholder(_) => {
                    panic!("Placeholders are only allowed in evm_asm_interpolator!")
                }
                AsmToken::Opcode(name) => {
                    // Check if this is a label reference
                    if defined_labels.contains(&name) {
                        quote! { emasm_common::AsmElement::Label(#name.to_string()) }
                    } else {
                        quote! { emasm_common::AsmElement::Opcode(#name.to_string()) }
                    }
                }
                AsmToken::Literal(val) => {
                    let bytes = val.to_be_bytes();
                    let trimmed: Vec<u8> = bytes.iter()
                        .skip_while(|&&b| b == 0)
                        .copied()
                        .collect();
                    quote! { emasm_common::AsmElement::Literal(vec![#(#trimmed),*]) }
                }
                AsmToken::HexLiteral(hex) => {
                    quote! { emasm_common::AsmElement::Literal(vec![#(#hex),*]) }
                }
                AsmToken::Label(name) => {
                    quote! { emasm_common::AsmElement::Label(#name.to_string()) }
                }
                AsmToken::Segment(name, inner) => {
                    let inner_tokens = inner.into_iter().map(|elem| match elem {
                        AsmToken::Opcode(n) => {
                            if defined_labels.contains(&n) {
                                quote! { emasm_common::AsmElement::Label(#n.to_string()) }
                            } else {
                                quote! { emasm_common::AsmElement::Opcode(#n.to_string()) }
                            }
                        },
                        AsmToken::Literal(v) => {
                            let bytes = v.to_be_bytes();
                            let trimmed: Vec<u8> = bytes.iter()
                                .skip_while(|&&b| b == 0)
                                .copied()
                                .collect();
                            quote! { emasm_common::AsmElement::Literal(vec![#(#trimmed),*]) }
                        }
                        AsmToken::HexLiteral(h) => quote! { emasm_common::AsmElement::Literal(vec![#(#h),*]) },
                        AsmToken::Label(n) => quote! { emasm_common::AsmElement::Label(#n.to_string()) },
                        AsmToken::BytesPtr(n) => quote! { emasm_common::AsmElement::BytesPtr(#n.to_string()) },
                        AsmToken::BytesSize(n) => quote! { emasm_common::AsmElement::BytesSize(#n.to_string()) },
                        _ => panic!("Nested segments not supported"),
                    });
                    quote! { 
                        emasm_common::AsmElement::Segment(
                            #name.to_string(), 
                            vec![#(#inner_tokens),*]
                        ) 
                    }
                }
                AsmToken::BytesSegment(name, data) => {
                    quote! { 
                        emasm_common::AsmElement::BytesSegment(#name.to_string(), vec![#(#data),*]) 
                    }
                }
                AsmToken::BytesPtr(name) => {
                    quote! { emasm_common::AsmElement::BytesPtr(#name.to_string()) }
                }
                AsmToken::BytesSize(name) => {
                    quote! { emasm_common::AsmElement::BytesSize(#name.to_string()) }
                }
            });

            let expanded = quote! {
                {
                    let elements = vec![#(#element_tokens),*];
                    let assembler = emasm_common::Assembler::new();
                    assembler.assemble(&elements).expect("Assembly failed")
                }
            };

            TokenStream::from(expanded)
        }
        Err(e) => {
            let error_msg = format!("Parse error: {}", e);
            TokenStream::from(quote! {
                compile_error!(#error_msg)
            })
        }
    }
}

#[proc_macro]
pub fn evm_asm_interpolator(input: TokenStream) -> TokenStream {
    let input_array = parse_macro_input!(input as ExprArray);
    
    match parse_asm_elements(&input_array.elems) {
        Ok(elements) => {
            // Collect all defined labels
            let mut defined_labels = std::collections::HashSet::new();
            fn collect_labels_interp(elem: &AsmToken, labels: &mut std::collections::HashSet<String>) {
                match elem {
                    AsmToken::Segment(name, inner) => {
                        labels.insert(name.clone());
                        for e in inner {
                            collect_labels_interp(e, labels);
                        }
                    }
                    AsmToken::BytesSegment(name, _) => {
                        labels.insert(name.clone());
                    }
                    _ => {}
                }
            }
            for elem in &elements {
                collect_labels_interp(elem, &mut defined_labels);
            }
            
            // First pass: count placeholders recursively
            fn count_placeholders(elem: &AsmToken) -> usize {
                match elem {
                    AsmToken::Placeholder(idx) => idx + 1,
                    AsmToken::Segment(_, inner) => {
                        inner.iter().map(count_placeholders).max().unwrap_or(0)
                    }
                    _ => 0,
                }
            }
            
            let placeholder_count = elements.iter()
                .map(count_placeholders)
                .max()
                .unwrap_or(0);
            
            let element_tokens = elements.into_iter().map(|elem| match elem {
                AsmToken::Placeholder(idx) => {
                    quote! { emasm_common::AsmElement::Placeholder(#idx) }
                }
                AsmToken::Opcode(name) => {
                    if defined_labels.contains(&name) {
                        quote! { emasm_common::AsmElement::Label(#name.to_string()) }
                    } else {
                        quote! { emasm_common::AsmElement::Opcode(#name.to_string()) }
                    }
                }
                AsmToken::Literal(val) => {
                    let bytes = val.to_be_bytes();
                    let trimmed: Vec<u8> = bytes.iter()
                        .skip_while(|&&b| b == 0)
                        .copied()
                        .collect();
                    quote! { emasm_common::AsmElement::Literal(vec![#(#trimmed),*]) }
                }
                AsmToken::HexLiteral(hex) => {
                    quote! { emasm_common::AsmElement::Literal(vec![#(#hex),*]) }
                }
                AsmToken::Label(name) => {
                    quote! { emasm_common::AsmElement::Label(#name.to_string()) }
                }
                AsmToken::Segment(name, inner) => {
                    let inner_tokens = inner.into_iter().map(|elem| match elem {
                        AsmToken::Placeholder(i) => {
                            quote! { emasm_common::AsmElement::Placeholder(#i) }
                        },
                        AsmToken::Opcode(n) => {
                            if defined_labels.contains(&n) {
                                quote! { emasm_common::AsmElement::Label(#n.to_string()) }
                            } else {
                                quote! { emasm_common::AsmElement::Opcode(#n.to_string()) }
                            }
                        },
                        AsmToken::Literal(v) => {
                            let bytes = v.to_be_bytes();
                            let trimmed: Vec<u8> = bytes.iter()
                                .skip_while(|&&b| b == 0)
                                .copied()
                                .collect();
                            quote! { emasm_common::AsmElement::Literal(vec![#(#trimmed),*]) }
                        }
                        AsmToken::HexLiteral(h) => quote! { emasm_common::AsmElement::Literal(vec![#(#h),*]) },
                        AsmToken::Label(n) => quote! { emasm_common::AsmElement::Label(#n.to_string()) },
                        AsmToken::BytesPtr(n) => quote! { emasm_common::AsmElement::BytesPtr(#n.to_string()) },
                        AsmToken::BytesSize(n) => quote! { emasm_common::AsmElement::BytesSize(#n.to_string()) },
                        _ => panic!("Nested segments not supported"),
                    });
                    quote! { 
                        emasm_common::AsmElement::Segment(
                            #name.to_string(), 
                            vec![#(#inner_tokens),*]
                        ) 
                    }
                }
                AsmToken::BytesSegment(name, data) => {
                    quote! { 
                        emasm_common::AsmElement::BytesSegment(#name.to_string(), vec![#(#data),*]) 
                    }
                }
                AsmToken::BytesPtr(name) => {
                    quote! { emasm_common::AsmElement::BytesPtr(#name.to_string()) }
                }
                AsmToken::BytesSize(name) => {
                    quote! { emasm_common::AsmElement::BytesSize(#name.to_string()) }
                }
            });

            let param_names: Vec<_> = (0..placeholder_count)
                .map(|i| syn::Ident::new(&format!("arg{}", i), proc_macro2::Span::call_site()))
                .collect();

            let expanded = quote! {
                {
                    use emasm_common::EVMEncodable;
                    
                    let template = vec![#(#element_tokens),*];
                    
                    move |#(#param_names: Box<dyn EVMEncodable>),*| -> Vec<u8> {
                        let values: Vec<Box<dyn EVMEncodable>> = vec![#(#param_names),*];
                        
                        fn substitute_elem(
                            elem: &emasm_common::AsmElement,
                            values: &[Box<dyn EVMEncodable>]
                        ) -> emasm_common::AsmElement {
                            match elem {
                                emasm_common::AsmElement::Placeholder(idx) => {
                                    emasm_common::AsmElement::Literal(values[*idx].to_evm_bytes())
                                }
                                emasm_common::AsmElement::Segment(label, inner) => {
                                    let substituted: Vec<_> = inner.iter()
                                        .map(|e| substitute_elem(e, values))
                                        .collect();
                                    emasm_common::AsmElement::Segment(label.clone(), substituted)
                                }
                                other => other.clone(),
                            }
                        }
                        
                        let result: Vec<_> = template.iter()
                            .map(|elem| substitute_elem(elem, &values))
                            .collect();
                        
                        let assembler = emasm_common::Assembler::new();
                        assembler.assemble(&result).expect("Assembly failed")
                    }
                }
            };

            TokenStream::from(expanded)
        }
        Err(e) => {
            let error_msg = format!("Parse error: {}", e);
            TokenStream::from(quote! {
                compile_error!(#error_msg)
            })
        }
    }
}
