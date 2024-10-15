use std::{fs, sync::mpsc};

use alloy::{
    hex::{FromHex, ToHexExt},
    signers::local::PrivateKeySigner,
};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{bytes::Bytes, Address, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::sol;

sol! {
    #[sol(rpc)] // <-- Important! Generates the necessary `MyContract` struct and function methods.
    contract Verifier {
        constructor(address) {} // The `deploy` method will also include any constructor arguments.

        #[derive(Debug)]
        function verifyProof(
            bytes32 programVKey,
            bytes calldata publicValues,
            bytes calldata proofBytes
        ) external;
    }
}

pub async fn post_to_l1(rx: mpsc::Receiver<u64>) {
    let contract_address = std::env::var("CONTRACT_ADDRESS").unwrap();
    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let rpc_url = std::env::var("RPC_URL").unwrap();
    let contract_address = Address::from_hex(contract_address).unwrap();
    let signer = PrivateKeySigner::from_bytes(&B256::from_hex(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);

    loop {
        let block_number = rx.recv();
        match block_number {
            Ok(bn) => {
                if bn == 0u64 {
                    continue;
                } else if bn == u64::MAX {
                    return;
                }
                let proof_file_path = format!("proofs/execution_proof_{}.proof", bn);
                let proof_file = fs::File::open(proof_file_path).unwrap();
                let proof_object: Result<sp1_sdk::SP1ProofWithPublicValues, serde_json::Error> =
                    serde_json::from_reader(proof_file);
                match proof_object {
                    Ok(proof) => {
                        _ = proof;
                        println!("proof found");
                        // if true {
                        //     return;
                        // }
                        // TODO: posting to L1 is done here.
                        let provider = ProviderBuilder::new()
                            .with_cached_nonce_management()
                            .wallet(wallet.clone())
                            .on_builtin(&rpc_url)
                            .await
                            .unwrap();
                        let contract = Verifier::new(contract_address, provider.clone());
                        let public_values =
                            Bytes::copy_from_slice(proof.public_values.raw().as_bytes());
                        println!(
                            "public values: {}, {:?}",
                            proof.public_values.raw(),
                            proof.proof.clone().try_as_plonk().unwrap().public_inputs
                        );
                        let plonk_proof = Bytes::copy_from_slice(
                            proof
                                .proof
                                .clone()
                                .try_as_plonk()
                                .unwrap()
                                .raw_proof
                                .as_bytes(),
                        );
                        println!(
                            "encoded proof: {}",
                            proof.proof.clone().try_as_plonk().unwrap().encoded_proof
                        );
                        let vkey = proof.proof.clone().try_as_plonk().unwrap().plonk_vkey_hash;
                        println!(
                            "vkey: {}",
                            proof
                                .proof
                                .try_as_plonk()
                                .unwrap()
                                .plonk_vkey_hash
                                .encode_hex()
                        );
                        let call_builder = contract.verifyProof(
                            alloy_primitives::FixedBytes(vkey),
                            alloy_primitives::Bytes(public_values),
                            alloy_primitives::Bytes(plonk_proof),
                        );
                        let call_builder = call_builder
                            .into_transaction_request()
                            .with_gas_limit(25000000)
                            .with_chain_id(1337)
                            .with_max_fee_per_gas(200000000000000)
                            .with_max_priority_fee_per_gas(2000000);
                        let return_value = provider.send_transaction(call_builder).await.unwrap();

                        let tx_hash = *return_value.tx_hash();

                        println!("tx hash is {:?}", tx_hash.encode_hex())
                    }
                    Err(_) => println!("proof not found"),
                }
            }
            Err(_) => break,
        }
    }
}
