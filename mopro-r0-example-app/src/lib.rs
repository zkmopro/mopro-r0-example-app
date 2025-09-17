// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This application demonstrates how to send an off-chain proof request
// to the Bonsai proving service and publish the received proofs directly
// to your deployed app contract.

// Allow unexpected cfg for the full file
#![allow(unexpected_cfgs)]

use methods::{RISC0_CIRCUIT_ELF, RISC0_CIRCUIT_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};

mopro_ffi::app!();

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum Risc0Error {
    #[error("Failed to prove: {0}")]
    ProveError(String),
    #[error("Failed to serialize receipt: {0}")]
    SerializeError(String),
    #[error("Failed to verify: {0}")]
    VerifyError(String),
    #[error("Failed to decode journal: {0}")]
    DecodeError(String),
}

#[derive(uniffi::Record, Clone)]
pub struct Risc0ProofOutput {
    pub receipt: Vec<u8>,
}

#[derive(uniffi::Record, Clone)]
pub struct Risc0VerifyOutput {
    pub is_valid: bool,
    pub output_value: u32,
}

#[uniffi::export]
pub fn risc0_prove(input: u32) -> Result<Risc0ProofOutput, Risc0Error> {
    // Create executor environment with input
    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|e| Risc0Error::ProveError(format!("Failed to write input: {}", e)))?
        .build()
        .map_err(|e| {
            Risc0Error::ProveError(format!("Failed to build executor environment: {}", e))
        })?;

    // Get the default prover
    let prover = default_prover();

    // Generate proof
    let prove_info = prover
        .prove(env, RISC0_CIRCUIT_ELF)
        .map_err(|e| Risc0Error::ProveError(format!("Failed to generate proof: {}", e)))?;

    // Extract receipt
    let receipt = prove_info.receipt;

    // Serialize receipt to bytes
    let receipt_bytes = bincode::serialize(&receipt)
        .map_err(|e| Risc0Error::SerializeError(format!("Failed to serialize receipt: {}", e)))?;

    Ok(Risc0ProofOutput {
        receipt: receipt_bytes,
    })
}

#[uniffi::export]
pub fn risc0_verify(receipt_bytes: Vec<u8>) -> Result<Risc0VerifyOutput, Risc0Error> {
    // Deserialize receipt from bytes
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)
        .map_err(|e| Risc0Error::SerializeError(format!("Failed to deserialize receipt: {}", e)))?;

    // Verify the receipt
    receipt
        .verify(RISC0_CIRCUIT_ID)
        .map_err(|e| Risc0Error::VerifyError(format!("Failed to verify receipt: {}", e)))?;

    // Extract output from journal
    let output_value: u32 = receipt
        .journal
        .decode()
        .map_err(|e| Risc0Error::DecodeError(format!("Failed to decode journal: {}", e)))?;

    Ok(Risc0VerifyOutput {
        is_valid: true,
        output_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risc0_prove_success() {
        // Test proving with a simple u32 input
        let input = 42u32;
        let result = risc0_prove(input);

        assert!(result.is_ok(), "Proving should succeed for valid input");

        let proof_output = result.unwrap();
        assert!(
            !proof_output.receipt.is_empty(),
            "Receipt should not be empty"
        );
    }

    #[test]
    fn test_risc0_verify_success() {
        // First generate a proof
        let input = 123u32;
        let prove_result = risc0_prove(input);
        assert!(prove_result.is_ok(), "Proving should succeed");

        let proof_output = prove_result.unwrap();

        // Now verify the proof
        let verify_result = risc0_verify(proof_output.receipt);
        assert!(
            verify_result.is_ok(),
            "Verification should succeed for valid proof"
        );

        let verify_output = verify_result.unwrap();
        assert!(verify_output.is_valid, "Proof should be valid");
        assert_eq!(
            verify_output.output_value, input,
            "Output value should match input"
        );
    }

    #[test]
    fn test_prove_verify_roundtrip() {
        // Test the complete prove -> verify workflow with multiple inputs
        let test_inputs = [0u32, 42u32, 100u32, 1000u32, u32::MAX];

        for &input in &test_inputs {
            // Generate proof
            let prove_result = risc0_prove(input);
            assert!(
                prove_result.is_ok(),
                "Proving should succeed for input: {}",
                input
            );

            let proof_output = prove_result.unwrap();

            // Verify proof
            let verify_result = risc0_verify(proof_output.receipt);
            assert!(
                verify_result.is_ok(),
                "Verification should succeed for input: {}",
                input
            );

            let verify_output = verify_result.unwrap();
            assert!(
                verify_output.is_valid,
                "Proof should be valid for input: {}",
                input
            );
            assert_eq!(
                verify_output.output_value, input,
                "Output should match input: {}",
                input
            );
        }
    }
}
