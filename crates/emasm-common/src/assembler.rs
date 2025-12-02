use crate::{
    opcodes::{opcode_map, Opcode},
    types::*,
    encodable::EVMEncodable,
};
use std::collections::HashMap;

pub struct Assembler {
    opcode_map: HashMap<&'static str, Opcode>,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            opcode_map: opcode_map(),
        }
    }

    pub fn assemble(&self, elements: &[AsmElement]) -> Result<Vec<u8>, AssemblerError> {
        let flattened = self.flatten(elements);
        let (label_map, bytes_map) = self.first_pass(&flattened)?;
        let optimized = self.optimize_labels(label_map, bytes_map, &flattened)?;
        self.encode(&flattened, &optimized.0, &optimized.1)
    }

    pub fn assemble_with_placeholders(
        &self,
        elements: &[AsmElement],
        placeholder_values: Vec<Box<dyn EVMEncodable>>,
    ) -> Result<Vec<u8>, AssemblerError> {
        let with_values = self.substitute_placeholders(elements, placeholder_values)?;
        self.assemble(&with_values)
    }

    fn substitute_placeholders(
        &self,
        elements: &[AsmElement],
        values: Vec<Box<dyn EVMEncodable>>,
    ) -> Result<Vec<AsmElement>, AssemblerError> {
        let mut result = Vec::new();
        for elem in elements {
            match elem {
                AsmElement::Placeholder(idx) => {
                    let value = values.get(*idx)
                        .ok_or(AssemblerError::InvalidPlaceholder(*idx))?;
                    result.push(AsmElement::Literal(value.to_evm_bytes()));
                }
                AsmElement::Segment(label, inner) => {
                    let inner_subst = self.substitute_placeholders(inner, values.iter().map(|_v| {
                        // This is a workaround - we'd need to clone Box<dyn EVMEncodable>
                        // For now we'll handle this differently in the proc macro
                        panic!("Cannot clone Box<dyn EVMEncodable>");
                    }).collect())?;
                    result.push(AsmElement::Segment(label.clone(), inner_subst));
                }
                _ => result.push(elem.clone()),
            }
        }
        Ok(result)
    }

    fn flatten(&self, elements: &[AsmElement]) -> Vec<AsmElement> {
        let mut result = Vec::new();
        for elem in elements {
            match elem {
                AsmElement::Segment(label, inner) => {
                    result.push(AsmElement::Segment(label.clone(), self.flatten(inner)));
                }
                _ => result.push(elem.clone()),
            }
        }
        result
    }

    fn first_pass(
        &self,
        elements: &[AsmElement],
    ) -> Result<(HashMap<String, LabelInfo>, HashMap<String, BytesInfo>), AssemblerError> {
        let mut labels = HashMap::new();
        let mut bytes_segments = HashMap::new();
        let mut offset = 0;

        for elem in elements {
            match elem {
                AsmElement::Segment(label, inner) => {
                    labels.insert(
                        label.clone(),
                        LabelInfo {
                            offset,
                            size_estimate: 2, // Initial estimate for PUSH address
                        },
                    );
                    offset += 1; // JUMPDEST
                    offset += self.estimate_size(inner, &labels, &bytes_segments);
                }
                AsmElement::BytesSegment(label, data) => {
                    bytes_segments.insert(
                        label.clone(),
                        BytesInfo {
                            offset,
                            size: data.len(),
                        },
                    );
                    offset += data.len();
                }
                AsmElement::Opcode(_) => offset += 1,
                AsmElement::Literal(data) => {
                    let push_len = if data.is_empty() || (data.len() == 1 && data[0] == 0) {
                        2 // PUSH1 0x00
                    } else {
                        let trimmed_len = data.iter().skip_while(|&&b| b == 0).count().max(1);
                        1 + trimmed_len
                    };
                    offset += push_len;
                }
                AsmElement::Label(_) => offset += 2, // Estimate PUSH1 (1) + 1-byte address (1)
                AsmElement::BytesPtr(_) | AsmElement::BytesSize(_) => offset += 2,
                AsmElement::Placeholder(_) => offset += 2, // Conservative estimate PUSH1 + data
            }
        }

        Ok((labels, bytes_segments))
    }

    fn estimate_size(
        &self,
        elements: &[AsmElement],
        labels: &HashMap<String, LabelInfo>,
        bytes_map: &HashMap<String, BytesInfo>,
    ) -> usize {
        let mut size = 0;
        for elem in elements {
            match elem {
                AsmElement::Segment(_, inner) => {
                    size += 1; // JUMPDEST
                    size += self.estimate_size(inner, labels, bytes_map);
                }
                AsmElement::Opcode(_) => size += 1,
                AsmElement::Literal(data) => size += 1 + data.len(),
                AsmElement::Label(label) => {
                    if let Some(info) = labels.get(label) {
                        size += 1 + info.size_estimate;
                    } else {
                        size += 3; // PUSH2 conservative
                    }
                }
                AsmElement::BytesPtr(_) | AsmElement::BytesSize(_) => size += 3,
                AsmElement::BytesSegment(_, data) => size += data.len(),
                AsmElement::Placeholder(_) => size += 3,
            }
        }
        size
    }

    fn optimize_labels(
        &self,
        mut labels: HashMap<String, LabelInfo>,
        bytes_map: HashMap<String, BytesInfo>,
        elements: &[AsmElement],
    ) -> Result<(HashMap<String, LabelInfo>, HashMap<String, BytesInfo>), AssemblerError> {
        const MAX_ITERATIONS: usize = 100;
        
        for _ in 0..MAX_ITERATIONS {
            let prev_labels = labels.clone();
            labels = self.recalculate_offsets(elements, labels, &bytes_map)?;
            
            if prev_labels.iter().all(|(k, v)| {
                labels.get(k).map(|new_v| new_v.offset == v.offset).unwrap_or(false)
            }) {
                return Ok((labels, bytes_map));
            }
        }
        
        Err(AssemblerError::CircularDependency)
    }

    fn recalculate_offsets(
        &self,
        elements: &[AsmElement],
        mut labels: HashMap<String, LabelInfo>,
        bytes_map: &HashMap<String, BytesInfo>,
    ) -> Result<HashMap<String, LabelInfo>, AssemblerError> {
        let mut offset = 0;

        for elem in elements {
            match elem {
                AsmElement::Segment(label, inner) => {
                    // Label should point to where the JUMPDEST will be
                    let jumpdest_offset = offset;
                    let push_size = self.calculate_push_size(jumpdest_offset);
                    if let Some(info) = labels.get_mut(label) {
                        info.offset = jumpdest_offset;
                        info.size_estimate = push_size;
                    }
                    offset += 1; // JUMPDEST
                    offset += self.calculate_segment_size(inner, &labels, bytes_map);
                }
                AsmElement::BytesSegment(_, data) => {
                    offset += data.len();
                }
                AsmElement::Opcode(_) => offset += 1,
                AsmElement::Literal(data) => offset += 1 + data.len(),
                AsmElement::Label(l) => {
                    if let Some(info) = labels.get(l) {
                        offset += 1 + info.size_estimate;
                    } else {
                        offset += 3;
                    }
                }
                AsmElement::BytesPtr(_) | AsmElement::BytesSize(_) => {
                    offset += 3;
                }
                AsmElement::Placeholder(_) => offset += 3,
            }
        }

        Ok(labels)
    }

    fn calculate_segment_size(
        &self,
        elements: &[AsmElement],
        labels: &HashMap<String, LabelInfo>,
        bytes_map: &HashMap<String, BytesInfo>,
    ) -> usize {
        let mut size = 0;
        for elem in elements {
            match elem {
                AsmElement::Segment(_, inner) => {
                    size += 1; // JUMPDEST
                    size += self.calculate_segment_size(inner, labels, bytes_map);
                }
                AsmElement::Opcode(_) => size += 1,
                AsmElement::Literal(data) => size += 1 + data.len(),
                AsmElement::Label(l) => {
                    if let Some(info) = labels.get(l) {
                        size += 1 + info.size_estimate;
                    } else {
                        size += 3;
                    }
                }
                AsmElement::BytesPtr(l) => {
                    if let Some(info) = bytes_map.get(l) {
                        size += 1 + self.calculate_push_size(info.offset);
                    } else {
                        size += 3;
                    }
                }
                AsmElement::BytesSize(l) => {
                    if let Some(info) = bytes_map.get(l) {
                        size += 1 + self.calculate_push_size(info.size);
                    } else {
                        size += 3;
                    }
                }
                AsmElement::BytesSegment(_, data) => size += data.len(),
                AsmElement::Placeholder(_) => size += 3,
            }
        }
        size
    }

    fn calculate_push_size(&self, value: usize) -> usize {
        if value == 0 {
            return 1; // PUSH1 needs 1 byte of data
        }
        // Calculate how many bytes are needed to represent the value
        let bytes_needed = ((value.ilog2() as usize) / 8) + 1;
        bytes_needed.min(32)
    }

    fn encode(
        &self,
        elements: &[AsmElement],
        labels: &HashMap<String, LabelInfo>,
        bytes_map: &HashMap<String, BytesInfo>,
    ) -> Result<Vec<u8>, AssemblerError> {
        let mut bytecode = Vec::new();

        for elem in elements {
            match elem {
                AsmElement::Opcode(name) => {
                    let opcode = self.opcode_map.get(name.as_str())
                        .ok_or_else(|| AssemblerError::UnknownOpcode(name.clone()))?;
                    bytecode.push(opcode.0);
                }
                AsmElement::Literal(data) => {
                    self.encode_push(&mut bytecode, data);
                }
                AsmElement::Segment(_, inner) => {
                    bytecode.push(Opcode::JUMPDEST.0);
                    bytecode.extend(self.encode(inner, labels, bytes_map)?);
                }
                AsmElement::Label(label) => {
                    let info = labels.get(label)
                        .ok_or_else(|| AssemblerError::LabelNotFound(label.clone()))?;
                    self.encode_push_value(&mut bytecode, info.offset);
                }
                AsmElement::BytesSegment(_, data) => {
                    bytecode.extend(data);
                }
                AsmElement::BytesPtr(label) => {
                    let info = bytes_map.get(label)
                        .ok_or_else(|| AssemblerError::LabelNotFound(label.clone()))?;
                    self.encode_push_value(&mut bytecode, info.offset);
                }
                AsmElement::BytesSize(label) => {
                    let info = bytes_map.get(label)
                        .ok_or_else(|| AssemblerError::LabelNotFound(label.clone()))?;
                    self.encode_push_value(&mut bytecode, info.size);
                }
                AsmElement::Placeholder(_) => {
                    return Err(AssemblerError::InvalidPlaceholder(0));
                }
            }
        }

        Ok(bytecode)
    }

    fn encode_push(&self, bytecode: &mut Vec<u8>, data: &[u8]) {
        let trimmed = data.iter()
            .skip_while(|&&b| b == 0)
            .copied()
            .collect::<Vec<_>>();
        
        if trimmed.is_empty() {
            // For zero, use PUSH1 0x00 for compatibility
            bytecode.push(Opcode::PUSH1.0);
            bytecode.push(0x00);
            return;
        }
        
        let len = trimmed.len().min(32);
        bytecode.push(Opcode::PUSH1.0 - 1 + len as u8);
        bytecode.extend(&trimmed[..len]);
    }

    fn encode_push_value(&self, bytecode: &mut Vec<u8>, value: usize) {
        if value == 0 {
            // For zero, use PUSH1 0x00 for compatibility
            bytecode.push(Opcode::PUSH1.0);
            bytecode.push(0x00);
            return;
        }

        let bytes = value.to_be_bytes();
        let trimmed = bytes.iter()
            .skip_while(|&&b| b == 0)
            .copied()
            .collect::<Vec<_>>();
        
        let len = trimmed.len().min(32);
        bytecode.push(Opcode::PUSH1.0 - 1 + len as u8);
        bytecode.extend(&trimmed[..len]);
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}
