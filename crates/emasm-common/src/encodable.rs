use alloy_primitives::{Address, Bytes, FixedBytes, U256};

pub trait EVMEncodable {
    fn to_evm_bytes(&self) -> Vec<u8>;
}

impl EVMEncodable for u8 {
    fn to_evm_bytes(&self) -> Vec<u8> {
        vec![*self]
    }
}

impl EVMEncodable for u16 {
    fn to_evm_bytes(&self) -> Vec<u8> {
        let bytes = self.to_be_bytes();
        bytes.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(0).take(if *self == 0 { 1 } else { 0 }))
            .collect()
    }
}

impl EVMEncodable for u32 {
    fn to_evm_bytes(&self) -> Vec<u8> {
        let bytes = self.to_be_bytes();
        bytes.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(0).take(if *self == 0 { 1 } else { 0 }))
            .collect()
    }
}

impl EVMEncodable for u64 {
    fn to_evm_bytes(&self) -> Vec<u8> {
        let bytes = self.to_be_bytes();
        bytes.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(0).take(if *self == 0 { 1 } else { 0 }))
            .collect()
    }
}

impl EVMEncodable for u128 {
    fn to_evm_bytes(&self) -> Vec<u8> {
        let bytes = self.to_be_bytes();
        bytes.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(0).take(if *self == 0 { 1 } else { 0 }))
            .collect()
    }
}

impl EVMEncodable for U256 {
    fn to_evm_bytes(&self) -> Vec<u8> {
        let bytes = self.to_be_bytes::<32>();
        bytes.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(0).take(if self.is_zero() { 1 } else { 0 }))
            .collect()
    }
}

impl EVMEncodable for Address {
    fn to_evm_bytes(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl<const N: usize> EVMEncodable for FixedBytes<N> {
    fn to_evm_bytes(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl EVMEncodable for Bytes {
    fn to_evm_bytes(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl EVMEncodable for Vec<u8> {
    fn to_evm_bytes(&self) -> Vec<u8> {
        self.clone()
    }
}

impl EVMEncodable for &[u8] {
    fn to_evm_bytes(&self) -> Vec<u8> {
        self.to_vec()
    }
}
