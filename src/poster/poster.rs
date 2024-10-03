use alloy::{
    hex::{FromHex, ToHexExt},
    signers::local::PrivateKeySigner,
};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::sol;

sol! {
    #[sol(rpc)] // <-- Important! Generates the necessary `MyContract` struct and function methods.
    #[sol(bytecode = "6080604052348015600e575f80fd5b506102128061001c5f395ff3fe608060405234801561000f575f80fd5b506004361061004a575f3560e01c80633fb5c1cb1461004e5780638381f58a1461006a578063d09de08a14610088578063f2c9ecd814610092575b5f80fd5b61006860048036038101906100639190610115565b6100b0565b005b6100726100b9565b60405161007f919061014f565b60405180910390f35b6100906100be565b005b61009a6100d6565b6040516100a7919061014f565b60405180910390f35b805f8190555050565b5f5481565b5f808154809291906100cf90610195565b9190505550565b5f8054905090565b5f80fd5b5f819050919050565b6100f4816100e2565b81146100fe575f80fd5b50565b5f8135905061010f816100eb565b92915050565b5f6020828403121561012a576101296100de565b5b5f61013784828501610101565b91505092915050565b610149816100e2565b82525050565b5f6020820190506101625f830184610140565b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f61019f826100e2565b91507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff82036101d1576101d0610168565b5b60018201905091905056fea2646970667358221220fc2f623e42449062279b74795e00c806c8bf3b825abc71630e38f067d6162f5a64736f6c63430008190033")] // <-- Generates the `BYTECODE` static and the `deploy` method.
    contract Counter {
        constructor(address) {} // The `deploy` method will also include any constructor arguments.

        #[derive(Debug)]
        function setNumber(uint256 newNumber) public;

        #[derive(Debug)]
        function increament() public;

        #[derive(Debug)]
        function getNumber() public view returns (uint256);
    }
}

pub async fn post_to_l1() {
    let contract_address = std::env::var("CONTRACT_ADDRESS").unwrap();
    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let rpc_url = std::env::var("RPC_URL").unwrap();

    let contract_address = Address::from_hex(contract_address).unwrap();
    let signer = PrivateKeySigner::from_bytes(&B256::from_hex(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .with_cached_nonce_management()
        .wallet(wallet.clone())
        .on_builtin(&rpc_url)
        .await
        .unwrap();
    let contract = Counter::new(contract_address, provider.clone());

    let call_builder = contract.getNumber();
    let number = call_builder.call().await.unwrap();
    println!("number is {}", number._0);

    let set_call_builder = contract
        .setNumber(U256::from(10))
        .into_transaction_request()
        .with_gas_limit(250000)
        .with_chain_id(1337)
        .with_max_fee_per_gas(2000000000000)
        .with_max_priority_fee_per_gas(2000000);

    let builder = provider.send_transaction(set_call_builder).await.unwrap();

    let tx_hash = *builder.tx_hash();

    println!("{:?}", tx_hash.encode_hex())
}
